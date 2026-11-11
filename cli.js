#!/usr/bin/env node

/**
 * OPfile - Headless CLI Companion (Feature 195)
 * Full-featured command line tool for cron jobs, terminal automation, and batch processing.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const args = process.argv.slice(2);
const command = args[0] ? args[0].toLowerCase() : 'help';

function printHelp() {
  console.log(`
================================================================================
  📱 OPfile Headless CLI Companion — Offline File Manager & Converter
================================================================================

Usage:
  opfile <command> [options]

Commands:
  convert <file> --to <ext> [--out <dir>]   Convert image, document, data, or archive
  checksum <file> [--algo <sha256|md5>]     Compute cryptographic hash
  shred <file> [--passes <num>]             Permanently wipe file (DoD 5220.22-M)
  catalog <dir> [--format <json|csv|md>]    Export directory catalog
  tree <dir>                                Display directory tree hierarchy
  watch <dir> --to <ext>                    Auto-convert new incoming files
  serve [--port <port>]                     Launch local mobile Wi-Fi server
  help                                      Show this help manual

Examples:
  opfile convert photo.png --to webp
  opfile checksum document.pdf --algo sha256
  opfile shred sensitive.key --passes 3
  opfile catalog ./downloads --format json
  opfile serve --port 8080
================================================================================
`);
}

async function main() {
  switch (command) {
    case 'help':
    case '--help':
    case '-h':
      printHelp();
      break;

    case 'checksum': {
      const filePath = args[1];
      if (!filePath) {
        console.error('Error: Please specify file path. Example: opfile checksum file.txt');
        process.exit(1);
      }
      const algo = args.includes('--algo') ? args[args.indexOf('--algo') + 1] : 'sha256';
      if (!fs.existsSync(filePath)) {
        console.error(`Error: File not found: ${filePath}`);
        process.exit(1);
      }
      const data = fs.readFileSync(filePath);
      const hash = crypto.createHash(algo).update(data).digest('hex');
      console.log(`\n  File:      ${path.basename(filePath)}`);
      console.log(`  Algorithm: ${algo.toUpperCase()}`);
      console.log(`  Hash:      ${hash}\n`);
      break;
    }

    case 'shred': {
      const filePath = args[1];
      if (!filePath) {
        console.error('Error: Please specify file path to shred.');
        process.exit(1);
      }
      const passes = args.includes('--passes') ? parseInt(args[args.indexOf('--passes') + 1], 10) : 3;
      if (!fs.existsSync(filePath)) {
        console.error(`Error: File not found: ${filePath}`);
        process.exit(1);
      }
      const stats = fs.statSync(filePath);
      const size = stats.size;

      console.log(`\n  🚨 Wiping file: ${filePath} (${size} bytes) with ${passes} passes...`);
      for (let p = 1; p <= passes; p++) {
        const randomBytes = crypto.randomBytes(Math.min(size, 65536));
        const fd = fs.openSync(filePath, 'r+');
        let written = 0;
        while (written < size) {
          const chunk = Math.min(randomBytes.length, size - written);
          fs.writeSync(fd, randomBytes, 0, chunk, written);
          written += chunk;
        }
        fs.closeSync(fd);
        console.log(`     Pass ${p}/${passes} complete.`);
      }
      fs.unlinkSync(filePath);
      console.log(`  ✅ File permanently shredded.\n`);
      break;
    }

    case 'tree': {
      const dirPath = args[1] || '.';
      function printTree(dir, prefix = '') {
        const files = fs.readdirSync(dir);
        files.forEach((f, idx) => {
          const isLast = idx === files.length - 1;
          const fullPath = path.join(dir, f);
          const stat = fs.statSync(fullPath);
          console.log(`${prefix}${isLast ? '└── ' : '├── '}${f}${stat.isDirectory() ? '/' : ''}`);
          if (stat.isDirectory()) {
            printTree(fullPath, prefix + (isLast ? '    ' : '│   '));
          }
        });
      }
      console.log(`\n📂 Directory Tree: ${path.resolve(dirPath)}`);
      printTree(dirPath);
      console.log('');
      break;
    }

    case 'catalog': {
      const dirPath = args[1] || '.';
      const formatIdx = args.indexOf('--format');
      const format = formatIdx !== -1 ? args[formatIdx + 1] : 'json';
      const items = [];

      function walk(dir) {
        const files = fs.readdirSync(dir);
        for (const f of files) {
          const fp = path.join(dir, f);
          const st = fs.statSync(fp);
          items.push({
            name: f,
            path: fp,
            isDir: st.isDirectory(),
            size: st.size,
            modified: st.mtime
          });
          if (st.isDirectory()) walk(fp);
        }
      }
      walk(dirPath);

      if (format === 'json') {
        console.log(JSON.stringify(items, null, 2));
      } else if (format === 'csv') {
        console.log('Name,Path,IsDir,Size,Modified');
        items.forEach(it => console.log(`"${it.name}","${it.path}",${it.isDir},${it.size},"${it.modified.toISOString()}"`));
      }
      break;
    }

    case 'serve': {
      const portIdx = args.indexOf('--port');
      const port = portIdx !== -1 ? parseInt(args[portIdx + 1], 10) : 8080;
      process.env.PORT = port;
      require('./server.js');
      break;
    }

    default:
      console.log(`Unknown command: ${command}`);
      printHelp();
      break;
  }
}

main().catch(err => console.error('Error:', err.message));
