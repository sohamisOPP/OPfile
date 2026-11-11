# 📱 OPfile — 100% Offline Mobile File Manager & Converter

**OPfile** is a fast, beautiful, and completely private mobile file manager and offline multi-format converter. It brings desktop-grade file conversion power and the iconic **Donut radial menu** straight to your phone.

---

## 🌟 Key Features

### 🍩 The Donut Converter
- Tap any file to open the circular **Donut Menu**.
- Slices dynamically highlight **only compatible output formats** for the chosen file.
- Interactive radial control with center cancel/close hole, pulsing feedback, and fine-tuning options.

### 🔒 100% Offline & Private
- **Zero internet required.** No file ever leaves your phone.
- No servers, no tracking, no external telemetry, and no file size limits.
- Persistent offline storage using sandboxed IndexedDB.

### 📂 Built-in Mobile File Manager
- **Browse & Organize:** Nested folder hierarchy with breadcrumbs (`Home / Documents / ...`), create new folders.
- **File Actions:** Preview, Rename, Delete, Duplicate, Download to Phone storage.
- **View Modes:** Toggle between Grid View and List View.
- **Search & Filters:** Real-time search + category pills (Images, PDF, Documents, Audio, Video, Archives, Data).
- **Batch Operations:** Multi-select files for batch deletion or one-tap ZIP archiving.

### 🔄 Multi-Format Offline Conversion Engines

| Category | Supported Conversions |
|---|---|
| **🖼️ Images** | PNG ↔ JPG/JPEG ↔ WebP ↔ BMP ↔ SVG ↔ ICO. Includes quality compression slider, max dimension resizing, 90°/180°/270° rotation, and black & white grayscale filter. |
| **📄 PDF & Documents** | Multi-Image to PDF (with A4 page fit or original sizing), PDF to high-res PNG/JPG images (auto-packaged in ZIP if multi-page), PDF merge, PDF page splitting, and PDF text extraction. |
| **🎵 Audio & Media** | Convert audio files (MP3, OGG, WebM) to clean uncompressed 16-bit PCM WAV. Extract audio soundtracks directly from video files. |
| **🎬 Video** | Extract snapshot still frames (JPEG or PNG) from video files. |
| **📦 Archives** | Compress files and folders into ZIP archives with configurable compression levels (1-9). Extract and unpack ZIP archives into local folders. |
| **📊 Data & Text** | JSON ↔ CSV spreadsheet, JSON ↔ XML, CSV ➔ HTML table, Markdown (.md) ➔ styled HTML. |

---

## 🚀 How to Run & Use on Your Phone

Run the local server:
```bash
node server.js
```

1. Connect your phone to the same Wi-Fi network as this PC.
2. Open your mobile browser (Chrome on Android, or Safari on iPhone).
3. Navigate to:
   ```
   http://<YOUR_LOCAL_IP>:8080
   ```
4. **Install as a Phone App:**
   - **Android (Chrome):** Tap the three-dot menu `⋮` and select **"Add to Home screen"** or **"Install app"**.
   - **iPhone (Safari):** Tap the Share button `⎋` and select **"Add to Home Screen"**.
5. The app will install onto your home screen with the official OPfile icon, launching in full-screen standalone mode and functioning **100% offline** even in Airplane mode!

---

## 🛠️ Project Structure

```
opfile/
├── server.js               # Zero-dependency local Node.js static server
├── README.md               # Documentation & mobile setup guide
├── .gitignore              # Git ignore rules
└── public/
    ├── index.html          # Mobile app shell with Fluent dark acrylic theme
    ├── manifest.json       # PWA manifest for standalone phone installation
    ├── sw.js               # Service Worker for 100% offline cache
    ├── icon.png            # OPfile application logo
    ├── css/
    │   └── style.css       # Mobile-first touch styles, Donut UI, and cards
    ├── js/
    │   ├── db.js           # IndexedDB persistent mobile storage
    │   ├── converters.js   # Offline conversion engines (Image, PDF, Audio, Archive, Data)
    │   ├── donut.js        # The signature circular Donut Menu
    │   ├── filemanager.js  # File browsing, folder tree, and file actions
    │   └── app.js          # App lifecycle, modals, and toasts
    └── libs/
        ├── jszip.min.js    # Local offline ZIP compression & extraction
        ├── pdf-lib.min.js  # Local offline PDF generation & merging
        ├── pdf.min.js      # Local offline PDF rendering
        └── pdf.worker.min.js
```

---

## 📜 License
MIT License. Open source and free for everyone.
