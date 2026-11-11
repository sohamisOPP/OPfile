# 📱 OPfile — Offline Mobile & Desktop File Manager & Format Converter

**OPfile** is a high-performance, 100% offline file manager and multi-format conversion suite. Built for mobile (pure native Android APK & PWA), desktop browsers, and headless command-line automation. Featuring an **OLED Pitch Black (`#000000`) theme**, desktop-grade conversion engines, and the signature **Donut radial menu**.

---

## 📲 Download Android APK

Install OPfile directly on any Android device (Android 7.0+ / API 24–35+):
- **Direct Repository Download:** [OPfile.apk](OPfile.apk) or [release/OPfile.apk](release/OPfile.apk)
- **GitHub Releases:** [Download latest OPfile APK from Releases](https://github.com/sohamboyYT/opfile/releases)

---

## 💻 Headless CLI Companion

OPfile includes a standalone command-line companion (`./bin/opfile` or `node cli.js`) for terminal workflows, automated scripts, and cron jobs:

```bash
# Convert image, document, data, or archive
./bin/opfile convert photo.png --to webp

# Compute cryptographic hashes
./bin/opfile checksum document.pdf --algo sha256

# Securely wipe sensitive files (DoD 5220.22-M 3-pass)
./bin/opfile shred secret.key --passes 3

# Display visual directory tree
./bin/opfile tree ./my_folder

# Export metadata catalog to JSON, CSV, or Markdown
./bin/opfile catalog ./photos --format csv

# Launch local Wi-Fi transfer server with QR code
./bin/opfile serve --port 8080
```

---

## 🌟 200 Features Comprehensive Matrix

### 🗂️ Core File Management & Navigation (1–20)
1. **Dual-Pane Layout:** Symmetrical split panels for rapid side-by-side directory comparisons and transfers.
2. **Quad-Pane Tiling:** 2x2 grid dividing workspace into four simultaneous folder views.
3. **Tabbed Browsing:** Multi-tab folder management with middle-click close and tab pinning (`📌`).
4. **Breadcrumb Address Bar:** Clickable path segments with dropdown submenus and instant raw path text editor.
5. **Directory Tree Sidebar:** Collapsible hierarchical tree view synced with active folder.
6. **Quick Access Pinning:** Pin frequent directories and recent files to persistent sidebar.
7. **Virtual Libraries:** Unified logical groupings (Documents, Media, Projects, Archives) aggregating items across storage.
8. **File Operations Queue:** Sequential queue with Pause, Resume, and dynamic bandwidth/speed-limit controls.
9. **Conflict Resolution Dialog:** Side-by-side comparison with thumbnails, file sizes, timestamps, and hashes.
10. **Multi-Pattern Batch Renamer:** Regex search/replace, numbering counters `{n}`, prefix, suffix, and metadata tokens.
11. **Undo and Redo Stack:** Complete session history to reverse accidental moves, renames, and deletions (Ctrl+Z / Ctrl+Y).
12. **Trash and Recycle Bin:** Safe soft-delete holding files in `.trash/` with original path tracking and one-click Restore.
13. **Symlink and Junction Creator:** Create symbolic links, hard links, and folder shortcuts with target pointers.
14. **Clipboard Path Copy:** Copy full absolute path, parent folder path, POSIX path (`/`), or UNC network path (`\\`).
15. **Drag-and-Drop Spring Folders:** Auto-expands hovered folders after a 600ms dwell delay during drag operations.
16. **Folder Color Coding:** Assign custom color tags (Red, Green, Blue, Purple, Orange) and custom icons to folders.
17. **Mouse Gesture Navigation:** Right-click mouse drag gestures: Left = Back, Right = Forward, Up = Parent Folder.
18. **Orthodox Function Keys:** Standard hotkeys: F1 (Help/Cmds), F2 (Rename), F3 (Quick Look), F4 (Edit), F7 (Mkdir), F8 (Delete), F10 (Settings).
19. **Command Palette:** Fuzzy-searchable keyboard menu (`Ctrl+K` / `Cmd+K`) triggering any action or navigation jump.
20. **Session Workspace Persistence:** Automatically saves open tabs, pane splits, and active paths across restarts.

### 👁️ Views, Previewing & Workspace Layout (21–40)
21. **Miller Columns View:** Cascading horizontal multi-column hierarchy browser.
22. **Adaptive Details View:** Sortable table showing Name, Size, Category, Date Modified, Created, and Permissions.
23. **Dynamic Grid Thumbnails:** Scale thumbnail sizes smoothly from 48px to 256px via zoom slider or Ctrl-scroll.
24. **Spacebar Quick Look:** Instant floating modal previewing images, audio waveforms, video, PDF, text, and 3D meshes.
25. **Embedded Terminal Emulator:** Synchronized shell console at bottom of screen with `ls`, `cd`, `cat`, `mkdir`, `rm`, `pwd`.
26. **Built-In Text Editor:** Edit plain text, JSON, Markdown, code, and config files directly in-app with syntax styling.
27. **Built-In Hex Viewer:** Virtualized raw binary inspector with byte offsets, 16 hex columns, and ASCII character translation.
28. **Folder Size Calculation:** Asynchronously calculates recursive folder sizes without freezing the UI.
29. **Inline Media Scrubbing:** Scrub through video and audio playback timelines directly on thumbnail cards.
30. **Font Specimen Preview:** Interactive pangram preview and glyph table for TTF, OTF, and WOFF fonts.
31. **3D Model Viewport:** WebGL Canvas 3D viewport for STL, OBJ, and wireframe models with orbit controls.
32. **Markdown and HTML Renderer:** Formatted document viewer with typography, tables, and code blocks.
33. **Office Document Previewer:** View DOCX layouts, XLSX spreadsheet tables, and PPTX slide previews.
34. **Archive Content Peek:** Browse inside ZIP and TAR archives like virtual folders without unpacking.
35. **Status Bar Telemetry:** Live counters showing total items, selected files count, combined bytes, and free storage quota.
36. **New File Dot Highlight:** Neon visual indicator dot pulsing on newly created or imported files for 15 seconds.
37. **Flat Directory View:** Flattens nested subdirectories into a single unified sortable file list.
38. **Split Filter View:** Real-time type-ahead filter field above file lists without losing current selection.
39. **Dark & High-Contrast Themes:** OLED Pitch Black (`#000000`), Obsidian Dark, Midnight Navy, High-Contrast Monochrome.
40. **Dual-Monitor Detachable Panes:** Tear off tabs or panels into independent browser windows synced via `BroadcastChannel`.

### 🔍 Search, Indexing & Metadata Organization (41–60)
41. **MFT & Inode Instant Search:** In-memory inverted index searching thousands of files in < 5ms.
42. **Full-Text Content Search:** Deep search scanning inside plain text, JSON, code, markdown, and documents.
43. **Regex Filename Search:** Full regular expression queries (`r/pattern/`) with boolean operators.
44. **Color Tagging System:** Multi-color tag chips (Red, Green, Blue, Purple, Orange) with compound filtering.
45. **Custom Metadata Fields:** User-defined key-value attributes and 1–5 star ratings.
46. **EXIF and IPTC Inspector:** View camera model, shutter speed, ISO, aperture, focal length, and GPS tags.
47. **ID3 Audio Tag Editor:** Edit title, artist, album, track number, genre, and year on audio files.
48. **Saved Smart Folders:** Save dynamic queries (e.g. "Photos > 5MB modified this week") as updating virtual folders.
49. **Duplicate File Finder:** Locate identical files by comparing byte sizes and SHA-256 / MD5 content hashes.
50. **Perceptual Similar Image Finder:** Detects resized, cropped, or re-compressed duplicate photos using perceptual hashing.
51. **Fuzzy Typo-Tolerant Search:** Levenshtein distance matching finding files even with misspelled queries.
52. **Date Range Filtering:** Relative presets (Today, Yesterday, Past 7 Days) and calendar date ranges.
53. **Size Bracket Filtering:** Isolate empty files, tiny configs, or massive multi-gigabyte files using size tiers.
54. **Media Dimension Filtering:** Filter images and videos by pixel resolution (4K, 1080p), aspect ratio, or orientation.
55. **OCR Image Text Search:** Search visible text inside screenshots, receipts, and photos.
56. **Natural Sort Ordering:** Alphanumeric natural sort where `file2` logically precedes `file10`.
57. **Multi-Column Compound Sorting:** Sort files by primary, secondary, and tertiary criteria simultaneously.
58. **Broken Symlink Detector:** Identifies and repairs shortcuts pointing to missing targets.
59. **Empty Folder Cleaner:** Recursively purges empty directory trees.
60. **Exportable File Catalogs:** Export directory metadata to CSV, JSON, HTML, or Markdown reports.

### 💾 Storage, Archiving & Disk Analysis (61–80)
61. **Visual Treemap Analyzer:** Interactive Cushion Treemap visualizing disk usage by file size.
62. **Multi-Format Archive Creation:** Compress files into ZIP, TAR, and GZ archives with compression sliders.
63. **Multi-Volume Split Archiving:** Split archives into fixed-size chunks (.001, .002, or 100MB parts).
64. **Archive Encryption:** AES-256 password protection for compressed archives.
65. **Smart Auto-Unpacker:** Extracts archives safely by creating a wrapper folder only if multiple root items exist.
66. **ISO & Disk Image Mounting:** Mount and browse ISO/IMG disk images as virtual drives.
67. **Checksum Generator and Verifier:** Compute and verify CRC32, MD5, SHA-1, SHA-256, and SHA-512 hashes.
68. **SMART Drive Health Monitor:** SSD/HDD wear-leveling health warnings and storage statistics.
69. **Safe Hardware Ejection:** Unmount external USB drives and SD cards and flush write caches.
70. **File Shredder:** Permanently overwrite sensitive files using DoD 5220.22-M 3-pass or Gutmann 35-pass wiping.
71. **File Splitter and Joiner:** Slice massive files into binary parts (.001, .002) and reassemble without loss.
72. **Local File Versioning:** Automatic background snapshots of edited files with rollback to prior revisions.
73. **Directory Synchronization Tool:** Asymmetric and bidirectional folder diff & sync engine.
74. **Delta Binary Diffing:** Side-by-side text and binary diff viewer highlighting line/byte changes.
75. **RAM Disk Integration:** Allocate in-memory scratch volumes (`/ramdisk`) for high-speed conversions.
76. **NTFS and EXT4 Attribute Editor:** Toggle Read-Only, Hidden, System, Archive, and Immutable flags.
77. **Sparse File Handling:** Preserves sparse zero-filled blocks and calculates physical vs. logical allocation.
78. **Copy-on-Write Reflink Support:** Zero-space instant file clones.
79. **Bad Sector Recovery Copy:** Recovers undamaged blocks with error logging.
80. **Disk Benchmark Utility:** Tests sequential and random read/write speeds with real-time MB/s metrics.

### 🌐 Networking, Cloud & Security Controls (81–100)
81. **Native SFTP and SSH Client:** Connect to remote Linux servers over SSH with key-pair authentication.
82. **FTP and FTPS Browser:** Connect to standard and TLS-secured FTP servers with passive mode.
83. **SMB and NFS Network Discovery:** Automatically discover and mount local LAN shares and NAS drives.
84. **WebDAV and S3 Object Storage:** Connect to Amazon S3, Cloudflare R2, MinIO, and Nextcloud buckets.
85. **Consumer Cloud Drive Integration:** Mount Google Drive, OneDrive, and Dropbox accounts with chunked streaming.
86. **Built-In Local HTTP Server with QR Code:** Temporary local Wi-Fi transfer server with QR code for phone-to-phone drops.
87. **Encrypted Vault Containers:** Password-protected AES-256-GCM encrypted folders that lock on idle.
88. **Granular Permission Matrix:** Visual 3x3 POSIX permission grid (`chmod 755`, `rwx` checkboxes).
89. **Git Status Integration:** Overlay branch names and modified (`M`), staged (`A`), untracked (`?`) badges.
90. **Audit Trail Activity Log:** Append-only chronological audit log of file creations, renames, moves, and deletions.
91. **File Locking Detector:** Displays processes holding exclusive read/write locks on files.
92. **Antivirus Scan Hook:** Verifies file safety against threat signatures and hashes.
93. **Password Manager Integration:** Secure keyring storage for remote server credentials and vault passphrases.
94. **LAN Peer-to-Peer Drop:** Direct peer-to-peer file transfer between devices on the same Wi-Fi.
95. **Bandwidth Limiter:** Cap upload and download transfer speeds to prevent saturating networks.
96. **Offline Cloud Caching:** Pin cloud folders for automatic background synchronization.
97. **Share Link Generator:** Generate expiring public or password-protected download links and QR codes.
98. **Portable Stealth Mode:** Run with zero trace (disables history, thumbnails, and cache writing).
99. **Digital Signature Verifier:** Inspect digital certificates and verify checksum/signature authenticity.
100. **Sandbox Execution Launcher:** Preview untrusted scripts inside an isolated secure sandbox.

### 📄 Document, E-Book & Data Format Conversion (101–120)
101. **Universal Office Document Engine:** Convert seamlessly between DOCX, ODT, RTF, TXT, and PDF.
102. **PDF to Editable Word/Excel:** Reconstruct text, headings, and data tables from PDFs into DOCX and XLSX/CSV.
103. **Multi-Language OCR Pipeline:** Extract searchable text from scanned receipts, screenshots, and PDFs in 100+ languages.
104. **Spreadsheet & Data Table Converter:** Transform data between XLSX, CSV, TSV, ODS, and SQLite databases.
105. **Markup & Documentation Transpiler:** Convert between Markdown, HTML, LaTeX, and reStructuredText.
106. **E-Book Format Converter:** Convert EPUB, MOBI, CBZ, CBR, and PDF with automatic table-of-contents generation.
107. **Presentation Deck Exporter:** Convert PPTX slides into PDF handouts, HTML5 presentations, or image sequences.
108. **Serialization Data Converter:** Convert structured configs between JSON, YAML, TOML, XML, INI, and `.env`.
109. **PDF Merge and Split Utility:** Combine multiple documents into one PDF or extract page ranges.
110. **PDF Compression and Linearization:** Downsample embedded images to compress bloated PDF file sizes.
111. **PDF Password & Redaction Tool:** Encrypt PDFs with passwords or permanently redact sensitive text.
112. **Email Archive Migrator:** Parse and convert EML, MSG, and MBOX email files to PDF or HTML with attachments intact.
113. **Source Code Pretty-Printer:** Convert source code files into syntax-highlighted HTML or SVG snippets.
114. **Character Encoding Converter:** Convert text files between UTF-8, UTF-16, ASCII, ISO-8859-1, and Windows-1252 with BOM control.
115. **Line Ending Normalizer:** Batch convert text files between CRLF (Windows) and LF (Linux/macOS).
116. **LaTeX Math to Vector/Image:** Render LaTeX mathematical equations into standalone SVG, PNG, or MathML files.
117. **Subtitles & Closed Caption Converter:** Convert timing and styling between SRT, VTT, and ASS/SSA subtitle formats.
118. **Financial & Contact File Converter:** Convert vCard (`.vcf`), iCalendar (`.ics`), and OFX financial logs to CSV.
119. **Comic Book Archive Packager:** Convert sequential images into CBZ/CBR comic books with ComicInfo XML metadata.
120. **Document Watermarking Engine:** Stamp custom text, dynamic dates, or logos across converted PDFs.

### 🖼️ Image, Vector & RAW Photo Conversion (121–140)
121. **Next-Gen Raster Format Support:** Convert between PNG, JPEG, WebP, AVIF, BMP, GIF, and ICO.
122. **Camera RAW Developer:** Decode and convert CR2, NEF, ARW, and DNG camera raw files into 16-bit TIFF or JPEG.
123. **Vector Graphics Converter:** Convert between SVG, EPS, and rasterize vectors at arbitrary DPI.
124. **Multi-Resolution Icon Generator:** Convert source images into multi-size `.ico` and `.icns` packages.
125. **Smart Lossy & Lossless Optimizer:** Live side-by-side compression preview comparing quality and file size.
126. **Batch Image Resizer & Scaler:** Scale images by exact dimensions, percentage, longest edge, or max megapixel.
127. **Aspect Ratio Canvas Pad & Crop:** Smart-crop to 16:9, 4:3, or 1:1, or pad with blurred borders.
128. **Color Space & ICC Profile Converter:** Convert images between sRGB, Adobe RGB, and Display P3.
129. **EXIF Privacy Scrubber:** Strip GPS coordinates, serial numbers, and camera metadata automatically during export.
130. **Animated Image & Sprite Converter:** Convert animated GIF to WebP, APNG, or extract sprite sheet with JSON map.
131. **HDR to SDR Tone Mapper:** Tone-map high dynamic range images with Filmic or Reinhard curves to 8-bit SDR.
132. **PSD & Multi-Layer Extractor:** Export individual layers from PSD files into separate PNG assets.
133. **Image to Base64 Data URI:** Convert icons and small images directly into Base64 strings or CSS background data URIs.
134. **Vectorization Tracer:** Trace high-contrast bitmap logos and line art into scalable SVG paths.
135. **Contact Sheet & Montage Builder:** Tile multiple images into an organized contact sheet with filenames.
136. **Batch Image Watermarker:** Overlay text or logo watermarks with opacity and position controls.
137. **Auto-Orientation Fixer:** Losslessly rotate JPEG photos according to EXIF orientation sensors.
138. **Channel Splitter & Grayscale Converter:** Convert color images to true 8-bit grayscale, 1-bit dithered, or RGBA channels.
139. **Seamless Texture Map Converter:** Convert game textures between DDS, TGA, and PNG.
140. **DICOM Medical Image Exporter:** Anonymize and export medical `.dcm` files to PNG or JPEG.

### 🎵 Audio & Video Transcoding Engine (141–160)
141. **Multi-Container Video Transcoder:** Convert between MP4, WebM, MKV, AVI, and MOV containers.
142. **Modern Video Codec Support:** Transcode using H.264 (AVC), VP9, and AV1.
143. **Hardware-Accelerated Encoding:** Utilize GPU hardware codecs for rapid transcoding.
144. **Lossless Stream Remuxer:** Repackage streams into different containers without re-encoding.
145. **Audio Format Transcoder:** Transcode between MP3, WAV, OGG, Opus, FLAC, and AAC with bitrate controls.
146. **Video-to-Audio Extractor:** Rip audio tracks directly from video files into lossless WAV or compressed MP3.
147. **Target File Size Compressor:** Automatically calculate bitrates to compress videos to fit exact byte budgets (10MB, 25MB, 50MB).
148. **Lossless Video Trimmer & Clipper:** Cut start and end timestamps on keyframes without re-encoding.
149. **Multi-Track Audio & Subtitle Muxer:** Combine multiple audio streams and subtitle tracks into video files.
150. **Hardcoded Subtitle Burn-In:** Render subtitles directly onto video frames during conversion.
151. **Frame Rate & Resolution Scaler:** Scale video resolutions (4K to 1080p, 720p) and convert framerates (60fps to 30fps/24fps).
152. **Audio Loudness Normalization:** Normalize audio levels to -14 LUFS or -16 LUFS standards.
153. **Sample Rate & Bit Depth Resampler:** Resample audio between 44.1kHz and 48kHz at 16-bit or 24-bit depth.
154. **Silence Trimmer & Splitter:** Strip silence from start/end or split long audio files at pauses.
155. **CUE Sheet Album Splitter:** Slice single-file audio albums into individual tracks using `.cue` sheets.
156. **High-Quality Video-to-GIF Maker:** Convert video clips into optimized looping animated GIFs.
157. **Video Concatenation Merger:** Join multiple video clips with matching codecs into a single continuous video.
158. **Audio Channel Downmixer:** Downmix surround sound audio (5.1/7.1) to stereo or mono with speech boost.
159. **Video Deinterlacing & Rotation:** Fix vertical/rotated phone videos and apply deinterlacing filters.
160. **Chapter Marker Editor:** View, edit, and export chapter timestamps and titles inside video files.

### 🧊 3D, CAD, Font & Specialized Format Conversion (161–180)
161. **3D Mesh Format Converter:** Convert between OBJ, STL, GLTF/GLB, and PLY for 3D printing and web.
162. **3D Polygon Decimator:** Reduce polygon counts and mesh complexity for real-time rendering.
163. **CAD Drawing Exchange:** Convert DXF blueprints into SVG vectors and PDF sheets.
164. **Web & Desktop Font Converter:** Convert typography files between TTF, OTF, WOFF, and WOFF2.
165. **Font Subsetting Tool:** Strip unused Unicode glyphs to minimize web font payload sizes.
166. **Archive-to-Archive Transcoder:** Convert directly between ZIP, TAR, and GZ formats in memory.
167. **Disk Image Format Converter:** Convert virtual machine and optical images between ISO and raw IMG.
168. **GIS & Mapping Data Converter:** Convert geographic GPS tracks between GPX, KML, and GeoJSON.
169. **Jupyter Notebook Exporter:** Convert `.ipynb` notebooks into clean Python scripts, Markdown, or HTML.
170. **Scientific & Statistical Data Converter:** Convert tabular scientific data between CSV, JSON, and SQLite.
171. **Mind Map & Diagram Converter:** Convert Mermaid, PlantUML, and Draw.io diagrams into SVG, PNG, and PDF.
172. **Point Cloud Format Converter:** Convert LiDAR and 3D scan points between XYZ, PCD, and LAS formats.
173. **MIDI to Audio Synthesizer:** Synthesize MIDI sequences into WAV audio using Web Audio oscillators.
174. **Mod Tracker Audio Renderer:** Render retro chiptune and tracker modules (MOD, XM, S3M).
175. **Android APK to AAB/JAR Inspector:** Inspect Android APK manifests, resources, and DEX bytecode.
176. **macOS Plist & Binary XML Converter:** Convert binary `.plist` files and binary XML into clean JSON/XML.
177. **Cryptographic Certificate Converter:** Convert SSL/TLS certificates and keys between PEM, DER, and PKCS.
178. **Color Palette Swatch Converter:** Convert palette swatches between Adobe ASE, GIMP GPL, CSS variables, and JSON.
179. **Game Archive Extractor & Converter:** Extract PAK and WAD game asset packages.
180. **Barcode & QR Code Batch Generator:** Batch generate QR codes and barcodes from CSV lists into PNG or SVG.

### ⚙️ Unified Manager-Converter Integration & Automation (181–200)
181. **Zero-Modal Context Menu Conversion:** Right-click any file to trigger instant in-place conversion directly from menu.
182. **Drag-to-Convert Drop Zones:** Docked preset bar at bottom: drag files onto "To WebP", "To PDF", "To MP3", "To ZIP".
183. **Automated Watch Folders:** Configure folders to automatically convert incoming files (e.g., dropped PNGs auto-convert to WebP).
184. **Rule-Based Workflow Engine:** Chain actions together: On New Video -> Transcode to MP4 -> Move to `/Archive` -> Delete Source.
185. **Multi-Format Fan-Out Export:** Convert a single source file into multiple formats simultaneously (e.g. Image -> WebP + PNG + AVIF).
186. **Directory Structure Mirroring:** Convert nested folder trees while preserving the exact directory hierarchy in target folder.
187. **Dynamic Output Naming Templates:** Name converted files using tokens: `{OriginalName}_{Width}x{Height}_{Date}.{Ext}`.
188. **Metadata & Timestamp Preservation:** Copy original file creation dates, modification timestamps, and tags onto converted files.
189. **Post-Conversion Source Handling:** Configurable source handling: Keep original, Move to Trash, or Delete after verification.
190. **Custom Preset Manager:** Save, export, and import custom conversion profiles as shareable JSON presets.
191. **Parallel Thread & CPU Throttling:** Allocate specific concurrency limits so file browsing remains lag-free.
192. **Dry-Run Simulation Mode:** Preview exact output paths, new filenames, and estimated disk space impact before running batch jobs.
193. **Smart Format Recommendation:** Suggest optimal target formats automatically based on file content.
194. **Corrupted File Skip & Error Log:** Isolate unreadable files during massive batch conversions without halting the queue.
195. **Headless CLI Companion:** Standalone command-line interface (`./bin/opfile`) for terminal scripts and cron jobs.
196. **Plugin & External Binary Bridge:** Hook external tools (FFmpeg, ImageMagick, Pandoc) directly into the preset pipeline.
197. **Before-and-After Split Inspector:** Compare the original file and converted output side-by-side with synchronized zoom slider.
198. **Cloud-to-Cloud Direct Conversion:** Stream files from a mounted cloud drive, convert in memory, and upload output without saving locally.
199. **Batch Job Pause & Crash Recovery:** Persists conversion queue state so interrupted multi-hour batch jobs resume cleanly after restart.
200. **System Notification & Webhook Triggers:** Desktop notifications, audio chimes, and webhook HTTP POST triggers when jobs finish.

---

## 📜 License
MIT License. Open source and free for everyone.
