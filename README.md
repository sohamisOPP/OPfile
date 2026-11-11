# 📱 OPfile — 100% Offline Mobile File Manager & Converter

**OPfile** is a fast, high-performance, and completely private mobile file manager and offline multi-format converter. Built as a **100% Pure Native Flutter Android app** featuring a sleek **Matte OLED Pitch Black (`#000000`) theme**, desktop-grade conversion power, and the iconic **Donut radial menu** straight on your phone.

---

## 📲 Download Android APK

Install OPfile directly on any Android device (Android 7.0+ / API 24-35+):
- **Direct Repository Download:** [OPfile.apk](OPfile.apk) or [release/OPfile.apk](release/OPfile.apk)
- **GitHub Releases:** [Download latest OPfile APK from Releases](https://github.com/sohamboyYT/opfile/releases)

---

## 🌟 Key Features

### 🖤 Solid OLED Pitch Black UI
- **Zero Glassmorphism / Acrylic Blur:** Crisp `#FFFFFF` text on pure `#000000` pitch black background for infinite contrast and zero battery drain on OLED screens.
- **Solid High-Contrast Cards:** `#111111` card surfaces with subtle `#222222` borders and electric `#6C5CE7` violet accents.
- **Silky Smooth 120 FPS:** Pure native Flutter hardware-accelerated rendering.

### 🍩 The Donut Radial Converter
- Long-press or tap any file to open the circular **Donut Menu** rendered via high-performance `CustomPainter`.
- Slices dynamically highlight **only compatible output formats** for the chosen file.
- Center cancel hole with haptic feedback, angle interpolation, and customizable conversion dialogs.

### 🔒 100% Offline & Private
- **Zero internet required.** No file ever leaves your phone.
- No cloud uploads, no tracking, no external telemetry, and no file size limits.
- Uses Android's native Storage Access Framework (SAF) — no intrusive permissions required.

### 📂 Built-in Native Mobile File Manager
- **Import from Device:** Multi-file picker directly connects to your phone's storage, SD cards, and downloads.
- **Organize & Manage:** View file metadata, rename, delete, batch select, and export directly to your device's `Download/` folder.
- **View Modes:** One-tap toggle between Responsive Grid View and Detailed List View.
- **Search & Category Filters:** Instant search + filter chips (`All`, `🖼️ Images`, `📄 Documents`, `🎵 Audio`, `🎬 Video`, `📦 Archives`, `📊 Data`).
- **Native Sharing:** One-tap system share sheet to send converted files to WhatsApp, Telegram, Drive, etc.

### 🔄 Multi-Format Offline Conversion Engines (Pure Native Dart)

| Category | Supported Conversions & Processing |
|---|---|
| **🖼️ Images** | PNG ↔ JPG/JPEG ↔ WebP ↔ BMP ↔ ICO ↔ GIF. Features quality compression slider (10–100%), max dimension downscaling, 90°/180°/270° rotation, and black & white grayscale filter. |
| **📦 Archives** | Multi-file ZIP compression (`ZipEncoder`) with instant extraction (`ZipDecoder`). |
| **📊 Data & Text** | JSON ↔ CSV spreadsheet, JSON ↔ XML, CSV ➔ HTML table, Markdown (.md) ➔ formatted HTML. |

---

## 🛠️ Project Structure

```
opfile/
├── OPfile.apk              # Latest release APK ready to install
├── README.md               # Documentation & mobile setup guide
├── release/
│   └── OPfile.apk          # Release binary mirror
├── mobile/                 # 100% Pure Native Flutter Android App
│   ├── lib/
│   │   ├── main.dart       # OLED Pitch Black theme & app entry point
│   │   ├── models/
│   │   │   └── file_item.dart        # File metadata & category mappings
│   │   ├── services/
│   │   │   ├── conversion_engine.dart # Pure Dart offline conversion engines
│   │   │   └── file_service.dart     # Native file system, SAF picker, and export
│   │   ├── widgets/
│   │   │   ├── donut_menu.dart       # 120fps CustomPainter Donut radial menu
│   │   │   ├── conversion_dialog.dart# Quality sliders, options & export
│   │   │   └── file_card.dart        # High-contrast OLED grid/list file cards
│   │   └── screens/
│   │       └── home_screen.dart      # Main browser, search & batch actions
│   └── pubspec.yaml        # Flutter dependencies (image, archive, file_picker, share_plus)
└── public/                 # Lightweight companion web version
    ├── index.html          # OLED black web shell
    ├── css/style.css       # Solid OLED black theme styles
    └── js/                 # Web conversion engines & file manager
```

---

## 📜 License
MIT License. Open source and free for everyone.
