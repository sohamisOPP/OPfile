use crate::error::{OpResult, OpfileError};
use crate::task_manager::{tokio_rt, AtomicPartFile};
use serde::Serialize;
use std::fs;
use std::net::{IpAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

struct ServerControl {
    running: Arc<AtomicBool>,
    root_dir: PathBuf,
    http_port: u16,
    ftp_port: u16,
    local_ip: String,
}

static SERVER_STATE: OnceLock<Mutex<Option<ServerControl>>> = OnceLock::new();

fn state_lock() -> &'static Mutex<Option<ServerControl>> {
    SERVER_STATE.get_or_init(|| Mutex::new(None))
}

#[derive(Serialize)]
pub struct WifiServerStatus {
    pub running: bool,
    pub http_url: String,
    pub ftp_url: String,
    pub root_dir: String,
}

fn detect_lan_ip() -> String {
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0") {
        if sock.connect("192.168.1.1:80").is_ok() || sock.connect("8.8.8.8:80").is_ok() {
            if let Ok(addr) = sock.local_addr() {
                let ip = addr.ip();
                if let IpAddr::V4(v4) = ip {
                    if !v4.is_loopback() {
                        return v4.to_string();
                    }
                }
            }
        }
    }
    "127.0.0.1".to_string()
}

pub fn start_wifi_server(root_dir: &str, http_port: u16, ftp_port: u16) -> OpResult<WifiServerStatus> {
    let rt = tokio_rt()?;
    let root = PathBuf::from(root_dir);
    if !root.is_dir() {
        return Err(OpfileError::invalid("Shared root must be an existing folder"));
    }

    stop_wifi_server();

    let running = Arc::new(AtomicBool::new(true));
    let ip = detect_lan_ip();

    let (actual_http_port, actual_ftp_port) = rt.block_on(async {
        let http_listener = TcpListener::bind(("0.0.0.0", http_port))
            .await
            .or_else(|_| std::net::TcpListener::bind("0.0.0.0:0").and_then(|l| {
                l.set_nonblocking(true)?;
                TcpListener::from_std(l)
            }))?;
        let actual_http = http_listener.local_addr()?.port();

        let ftp_listener = TcpListener::bind(("0.0.0.0", ftp_port))
            .await
            .or_else(|_| std::net::TcpListener::bind("0.0.0.0:0").and_then(|l| {
                l.set_nonblocking(true)?;
                TcpListener::from_std(l)
            }))?;
        let actual_ftp = ftp_listener.local_addr()?.port();

        let run_http = Arc::clone(&running);
        let root_http = root.clone();
        tokio::spawn(async move {
            while run_http.load(Ordering::SeqCst) {
                match tokio::time::timeout(std::time::Duration::from_millis(250), http_listener.accept()).await {
                    Ok(Ok((stream, _))) => {
                        let r = root_http.clone();
                        tokio::spawn(async move {
                            let _ = handle_http_client(stream, r).await;
                        });
                    }
                    Ok(Err(_)) => break,
                    Err(_) => continue,
                }
            }
        });

        let run_ftp = Arc::clone(&running);
        let root_ftp = root.clone();
        let ip_ftp = ip.clone();
        tokio::spawn(async move {
            while run_ftp.load(Ordering::SeqCst) {
                match tokio::time::timeout(std::time::Duration::from_millis(250), ftp_listener.accept()).await {
                    Ok(Ok((stream, _))) => {
                        let r = root_ftp.clone();
                        let ip_c = ip_ftp.clone();
                        tokio::spawn(async move {
                            let _ = handle_ftp_client(stream, r, ip_c).await;
                        });
                    }
                    Ok(Err(_)) => break,
                    Err(_) => continue,
                }
            }
        });

        Ok::<(u16, u16), std::io::Error>((actual_http, actual_ftp))
    })?;

    if let Ok(mut guard) = state_lock().lock() {
        *guard = Some(ServerControl {
            running,
            root_dir: root,
            http_port: actual_http_port,
            ftp_port: actual_ftp_port,
            local_ip: ip.clone(),
        });
    }

    Ok(WifiServerStatus {
        running: true,
        http_url: format!("http://{ip}:{actual_http_port}"),
        ftp_url: format!("ftp://{ip}:{actual_ftp_port}"),
        root_dir: root_dir.to_string(),
    })
}

pub fn stop_wifi_server() -> WifiServerStatus {
    if let Ok(mut guard) = state_lock().lock() {
        if let Some(ctrl) = guard.take() {
            ctrl.running.store(false, Ordering::SeqCst);
        }
    }
    WifiServerStatus {
        running: false,
        http_url: String::new(),
        ftp_url: String::new(),
        root_dir: String::new(),
    }
}

pub fn get_wifi_server_status() -> WifiServerStatus {
    if let Ok(guard) = state_lock().lock() {
        if let Some(ref ctrl) = *guard {
            if ctrl.running.load(Ordering::SeqCst) {
                return WifiServerStatus {
                    running: true,
                    http_url: format!("http://{}:{}", ctrl.local_ip, ctrl.http_port),
                    ftp_url: format!("ftp://{}:{}", ctrl.local_ip, ctrl.ftp_port),
                    root_dir: ctrl.root_dir.to_string_lossy().into_owned(),
                };
            }
        }
    }
    WifiServerStatus {
        running: false,
        http_url: String::new(),
        ftp_url: String::new(),
        root_dir: String::new(),
    }
}

fn sanitize_rel(root: &Path, raw: &str) -> PathBuf {
    let trimmed = raw.trim_start_matches('/');
    let mut out = root.to_path_buf();
    for seg in Path::new(trimmed).components() {
        if let std::path::Component::Normal(s) = seg {
            out.push(s);
        }
    }
    out
}

fn sanitize_child(base: &Path, root: &Path, raw: &str) -> Option<PathBuf> {
    let mut out = base.to_path_buf();
    for seg in Path::new(raw.trim_start_matches('/')).components() {
        if let std::path::Component::Normal(s) = seg {
            out.push(s);
        }
    }
    if out.starts_with(root) {
        Some(out)
    } else {
        None
    }
}

async fn handle_http_client(stream: TcpStream, root: PathBuf) -> std::io::Result<()> {
    let mut reader = BufReader::new(stream);
    let mut req_line = String::new();
    if reader.read_line(&mut req_line).await? == 0 {
        return Ok(());
    }
    let parts: Vec<&str> = req_line.split_whitespace().collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0];
    let raw_path = parts[1].split('?').next().unwrap_or("/");
    let decoded = percent_decode(raw_path);

    let mut content_len = 0usize;
    loop {
        let mut h = String::new();
        let n = reader.read_line(&mut h).await?;
        if n == 0 || h == "\r\n" || h == "\n" {
            break;
        }
        if let Some((k, v)) = h.split_once(':') {
            if k.trim().eq_ignore_ascii_case("content-length") {
                content_len = v.trim().parse().unwrap_or(0);
            }
        }
    }

    let target = sanitize_rel(&root, &decoded);

    if method == "PUT" || method == "POST" {
        let mut body = vec![0u8; content_len.min(256 * 1024 * 1024)];
        reader.read_exact(&mut body).await?;
        let out_file = if target.is_dir() || decoded.ends_with('/') {
            target.join("upload.bin")
        } else {
            target
        };
        if let Ok((guard, mut w)) = AtomicPartFile::create(&out_file) {
            use std::io::Write;
            let _ = w.write_all(&body);
            let _ = w.flush();
            drop(w);
            let _ = guard.commit();
        }
        let mut stream = reader.into_inner();
        let resp = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK";
        stream.write_all(resp).await?;
        return Ok(());
    }

    let mut stream = reader.into_inner();
    if target.is_file() {
        let len = target.metadata().map(|m| m.len()).unwrap_or(0);
        let name = target.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let hdr = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {len}\r\nContent-Disposition: attachment; filename=\"{name}\"\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(hdr.as_bytes()).await?;
        let mut f = tokio::fs::File::open(&target).await?;
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            let n = f.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            stream.write_all(&buf[..n]).await?;
        }
    } else if target.is_dir() {
        let mut html = String::from("<!DOCTYPE html><html><head><meta charset='utf-8'><title>PrismFiles</title><style>body{background:#000;color:#fff;font-family:sans-serif;padding:16px}a{color:#6C5CE7;text-decoration:none;display:block;padding:8px 0;border-bottom:1px solid #222}</style></head><body><h2>PrismFiles Transfer</h2>");
        if let Ok(rd) = fs::read_dir(&target) {
            for e in rd.filter_map(Result::ok) {
                let name = e.file_name().to_string_lossy().into_owned();
                let rel = if decoded == "/" {
                    format!("/{name}")
                } else {
                    format!("{}/{name}", decoded.trim_end_matches('/'))
                };
                html.push_str(&format!("<a href=\"{rel}\">{name}</a>"));
            }
        }
        html.push_str("</body></html>");
        let hdr = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            html.len()
        );
        stream.write_all(hdr.as_bytes()).await?;
        stream.write_all(html.as_bytes()).await?;
    } else {
        stream
            .write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nNot Found")
            .await?;
    }
    Ok(())
}

async fn handle_ftp_client(stream: TcpStream, root: PathBuf, ip: String) -> std::io::Result<()> {
    let (r, mut w) = stream.into_split();
    let mut reader = BufReader::new(r);
    w.write_all(b"220 PrismFiles Offline FTP Ready\r\n").await?;

    let mut cwd = root.clone();
    let mut pasv_listener: Option<TcpListener> = None;

    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let (cmd, arg) = match trimmed.split_once(' ') {
            Some((c, a)) => (c.to_ascii_uppercase(), a.trim().to_string()),
            None => (trimmed.to_ascii_uppercase(), String::new()),
        };

        match cmd.as_str() {
            "USER" => w.write_all(b"331 Anonymous ok, send password\r\n").await?,
            "PASS" => w.write_all(b"230 Login successful\r\n").await?,
            "SYST" => w.write_all(b"215 UNIX Type: L8\r\n").await?,
            "FEAT" => w.write_all(b"211-Features:\r\n UTF8\r\n211 End\r\n").await?,
            "TYPE" => w.write_all(b"200 Type set\r\n").await?,
            "PWD" | "XPWD" => {
                let rel = cwd.strip_prefix(&root).unwrap_or(Path::new(""));
                let p = format!("/{}", rel.to_string_lossy().replace('\\', "/"));
                w.write_all(format!("257 \"{p}\" is current directory\r\n").as_bytes()).await?;
            }
            "CWD" => {
                let next = if arg.starts_with('/') {
                    sanitize_rel(&root, &arg)
                } else {
                    sanitize_child(&cwd, &root, &arg).unwrap_or_else(|| root.clone())
                };
                if next.is_dir() && next.starts_with(&root) {
                    cwd = next;
                    w.write_all(b"250 Directory changed\r\n").await?;
                } else {
                    w.write_all(b"550 Directory not found\r\n").await?;
                }
            }
            "CDUP" => {
                if let Some(p) = cwd.parent() {
                    if p.starts_with(&root) {
                        cwd = p.to_path_buf();
                    }
                }
                w.write_all(b"200 Directory up\r\n").await?;
            }
            "PASV" => {
                let l = TcpListener::bind("0.0.0.0:0").await?;
                let port = l.local_addr()?.port();
                let ip_parts = ip.replace('.', ",");
                let p1 = port / 256;
                let p2 = port % 256;
                pasv_listener = Some(l);
                w.write_all(format!("227 Entering Passive Mode ({ip_parts},{p1},{p2})\r\n").as_bytes())
                    .await?;
            }
            "LIST" | "NLST" => {
                if let Some(l) = pasv_listener.take() {
                    w.write_all(b"150 Opening data connection for listing\r\n").await?;
                    if let Ok((mut data, _)) = l.accept().await {
                        if let Ok(rd) = fs::read_dir(&cwd) {
                            for e in rd.filter_map(Result::ok) {
                                let name = e.file_name().to_string_lossy().into_owned();
                                let meta = e.metadata().ok();
                                let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
                                let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                                let line_out = if cmd == "NLST" {
                                    format!("{name}\r\n")
                                } else if is_dir {
                                    format!("drwxr-xr-x 1 prismfiles prismfiles {size} Jan 01 00:00 {name}\r\n")
                                } else {
                                    format!("-rw-r--r-- 1 prismfiles prismfiles {size} Jan 01 00:00 {name}\r\n")
                                };
                                let _ = data.write_all(line_out.as_bytes()).await;
                            }
                        }
                    }
                    w.write_all(b"226 Transfer complete\r\n").await?;
                } else {
                    w.write_all(b"425 Use PASV first\r\n").await?;
                }
            }
            "RETR" => {
                let target_opt = if arg.starts_with('/') {
                    Some(sanitize_rel(&root, &arg))
                } else {
                    sanitize_child(&cwd, &root, &arg)
                };
                let target = match target_opt {
                    Some(t) => t,
                    None => {
                        w.write_all(b"550 Access denied\r\n").await?;
                        continue;
                    }
                };
                if let Some(l) = pasv_listener.take() {
                    if target.is_file() {
                        w.write_all(b"150 Opening binary data connection\r\n").await?;
                        if let Ok((mut data, _)) = l.accept().await {
                            if let Ok(mut f) = tokio::fs::File::open(&target).await {
                                let _ = tokio::io::copy(&mut f, &mut data).await;
                            }
                        }
                        w.write_all(b"226 Transfer complete\r\n").await?;
                    } else {
                        w.write_all(b"550 File not found\r\n").await?;
                    }
                } else {
                    w.write_all(b"425 Use PASV first\r\n").await?;
                }
            }
            "STOR" => {
                let target_opt = if arg.starts_with('/') {
                    Some(sanitize_rel(&root, &arg))
                } else {
                    sanitize_child(&cwd, &root, &arg)
                };
                let target = match target_opt {
                    Some(t) => t,
                    None => {
                        w.write_all(b"550 Access denied\r\n").await?;
                        continue;
                    }
                };
                if let Some(l) = pasv_listener.take() {
                    w.write_all(b"150 Ready to receive file\r\n").await?;
                    if let Ok((mut data, _)) = l.accept().await {
                        if let Ok((guard, mut bw)) = AtomicPartFile::create(&target) {
                            let mut buf = vec![0u8; 64 * 1024];
                            use std::io::Write;
                            while let Ok(n) = data.read(&mut buf).await {
                                if n == 0 {
                                    break;
                                }
                                let _ = bw.write_all(&buf[..n]);
                            }
                            let _ = bw.flush();
                            drop(bw);
                            let _ = guard.commit();
                        }
                    }
                    w.write_all(b"226 Store complete\r\n").await?;
                } else {
                    w.write_all(b"425 Use PASV first\r\n").await?;
                }
            }
            "QUIT" => {
                w.write_all(b"221 Goodbye\r\n").await?;
                break;
            }
            _ => w.write_all(b"502 Command not implemented\r\n").await?,
        }
    }
    Ok(())
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(b) = u8::from_str_radix(hex_str, 16) {
                    out.push(b);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
