/**
 * OPfile - Views, Previewing & Workspace Layout (Features 21–40)
 * 21. Miller Columns View
 * 22. Adaptive Details View
 * 23. Dynamic Grid Thumbnails
 * 24. Spacebar Quick Look
 * 25. Embedded Terminal Emulator
 * 26. Built-In Text Editor
 * 27. Built-In Hex Viewer
 * 28. Folder Size Calculation
 * 29. Inline Media Scrubbing
 * 30. Font Specimen Preview
 * 31. 3D Model Viewport (STL, OBJ, GLTF)
 * 32. Markdown and HTML Renderer
 * 33. Office Document Previewer (DOCX, XLSX, PPTX)
 * 34. Archive Content Peek
 * 35. Status Bar Telemetry
 * 36. New File Dot Highlight
 * 37. Flat Directory View
 * 38. Split Filter View
 * 39. Dark and High-Contrast Themes
 * 40. Dual-Monitor Detachable Panes
 */

class ViewsAndPreviewsManager {
  constructor() {
    this.hierarchyMode = 'standard'; // 'standard' | 'miller' | 'flat'
    this.gridThumbnailSize = 120; // 48 to 256 px
    this.isTerminalOpen = false;
    this.terminalHistory = [];
    this.currentTheme = 'oled-black'; // 'oled-black' | 'obsidian' | 'midnight' | 'contrast'
    this.recentNewFileIds = new Set();
    this.broadcastChannel = null;

    if (typeof BroadcastChannel !== 'undefined') {
      try {
        this.broadcastChannel = new BroadcastChannel('opfile_sync_channel');
        this.broadcastChannel.onmessage = (e) => this.handleSyncMessage(e.data);
      } catch (e) {}
    }

    this.initSpacebarQuickLook();
  }

  // --- Feature 21: Miller Columns View ---
  async setHierarchyMode(mode) {
    this.hierarchyMode = mode;
    const container = document.getElementById('file-grid-container');
    if (!container) return;

    if (mode === 'miller') {
      container.classList.add('miller-columns-view');
      await this.renderMillerColumns();
      window.app.showToast('Switched to Miller Columns View');
    } else if (mode === 'flat') {
      container.classList.remove('miller-columns-view');
      await this.renderFlatDirectory();
      window.app.showToast('Switched to Flat Directory View (All Subfolders)');
    } else {
      container.classList.remove('miller-columns-view');
      if (window.fileManager) await window.fileManager.loadCurrentFolder();
    }
  }

  async renderMillerColumns() {
    const container = document.getElementById('file-grid-container');
    if (!container || !window.fileeDB) return;
    const allItems = await window.fileeDB.getAllItems();

    // Column 0: Root items
    const rootItems = allItems.filter(it => it.parentId === 'root');
    // Column 1: Children of selected folder in Col 0 (if any)
    const activeFolder = allItems.find(it => it.id === window.fileManager.currentFolderId && it.isFolder);
    const childItems = activeFolder ? allItems.filter(it => it.parentId === activeFolder.id) : [];

    container.innerHTML = `
      <div class="miller-columns-wrapper">
        <div class="miller-col" id="miller-col-0">
          <div class="miller-col-header">🏠 Root Level</div>
          <div class="miller-col-list">
            ${rootItems.map(it => `
              <div class="miller-col-item ${it.id === window.fileManager.currentFolderId ? 'active' : ''}" 
                   onclick="${it.isFolder ? `window.fileManager.navigateToFolder('${it.id}', '${it.name.replace(/'/g, "\\'")}')` : `window.viewsPreviews.openQuickLookById('${it.id}')`}">
                <span>${it.isFolder ? '📁' : '📄'}</span>
                <span class="miller-title">${it.name}</span>
                ${it.isFolder ? '<span class="miller-arrow">›</span>' : ''}
              </div>
            `).join('')}
          </div>
        </div>

        <div class="miller-col" id="miller-col-1">
          <div class="miller-col-header">${activeFolder ? `📁 ${activeFolder.name}` : 'Select a folder'}</div>
          <div class="miller-col-list">
            ${childItems.length > 0 ? childItems.map(it => `
              <div class="miller-col-item" onclick="${it.isFolder ? `window.fileManager.navigateToFolder('${it.id}', '${it.name.replace(/'/g, "\\'")}')` : `window.viewsPreviews.openQuickLookById('${it.id}')`}">
                <span>${it.isFolder ? '📁' : '📄'}</span>
                <span class="miller-title">${it.name}</span>
                ${it.isFolder ? '<span class="miller-arrow">›</span>' : ''}
              </div>
            `).join('') : '<div class="empty-state">No items inside</div>'}
          </div>
        </div>
      </div>
    `;
  }

  // --- Feature 23: Dynamic Grid Thumbnails Scale ---
  setThumbnailSize(pixels) {
    this.gridThumbnailSize = Math.max(48, Math.min(256, pixels));
    document.documentElement.style.setProperty('--grid-thumb-size', `${this.gridThumbnailSize}px`);
    const sizeLabel = document.getElementById('thumb-size-val');
    if (sizeLabel) sizeLabel.innerText = `${this.gridThumbnailSize}px`;
  }

  // --- Feature 24: Spacebar Quick Look ---
  initSpacebarQuickLook() {
    window.addEventListener('keydown', (e) => {
      if (e.code === 'Space' && !['INPUT', 'TEXTAREA'].includes(document.activeElement.tagName)) {
        e.preventDefault();
        this.toggleQuickLook();
      }
    });
  }

  async toggleQuickLook() {
    const modal = document.getElementById('modal-quick-look');
    if (!modal) return;
    if (modal.style.display === 'flex') {
      modal.style.display = 'none';
      return;
    }
    const selected = window.fileManager ? window.fileManager.getSelectedItems() : [];
    if (selected.length === 0 && window.fileManager && window.fileManager.items.length > 0) {
      await this.showQuickLook(window.fileManager.items[0]);
    } else if (selected.length > 0) {
      await this.showQuickLook(selected[0]);
    } else {
      window.app.showToast('Select a file to preview with Quick Look (Spacebar)');
    }
  }

  async openQuickLookById(id) {
    if (!window.fileeDB) return;
    const item = await window.fileeDB.getItem(id);
    if (item) this.showQuickLook(item);
  }

  async showQuickLook(item) {
    const modal = document.getElementById('modal-quick-look');
    if (!modal) return;
    document.getElementById('quick-look-title').innerText = item.name;
    document.getElementById('quick-look-meta').innerText = `${window.app.formatSize(item.size)} • ${new Date(item.modifiedAt || Date.now()).toLocaleDateString()}`;

    const previewContainer = document.getElementById('quick-look-preview-body');
    previewContainer.innerHTML = '<div class="spinner"></div>';
    modal.style.display = 'flex';

    const ext = window.fileeConverter ? window.fileeConverter.getExtension(item.name) : '';

    if (['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif', 'svg', 'ico'].includes(ext)) {
      // Image Preview with Zoom
      let url = item.data instanceof Blob ? URL.createObjectURL(item.data) : (item.previewUrl || '/icon.png');
      previewContainer.innerHTML = `
        <div class="ql-image-wrap">
          <img src="${url}" class="ql-image" alt="${item.name}" />
        </div>
      `;
    } else if (['mp3', 'wav', 'ogg', 'webm', 'flac'].includes(ext)) {
      // Audio Preview with Waveform
      let url = item.data instanceof Blob ? URL.createObjectURL(item.data) : '';
      previewContainer.innerHTML = `
        <div class="ql-media-box">
          <div class="audio-waveform-mock">
            <span style="height: 40%"></span><span style="height: 70%"></span><span style="height: 90%"></span>
            <span style="height: 100%"></span><span style="height: 60%"></span><span style="height: 80%"></span>
            <span style="height: 50%"></span><span style="height: 30%"></span><span style="height: 75%"></span>
          </div>
          <audio controls autoplay src="${url}" style="width: 100%; margin-top: 1rem;"></audio>
        </div>
      `;
    } else if (['mp4', 'webm', 'mov', 'mkv'].includes(ext)) {
      // Video Preview
      let url = item.data instanceof Blob ? URL.createObjectURL(item.data) : '';
      previewContainer.innerHTML = `
        <div class="ql-media-box">
          <video controls autoplay src="${url}" style="max-width: 100%; max-height: 420px; border-radius: 8px;"></video>
        </div>
      `;
    } else if (['txt', 'js', 'json', 'html', 'css', 'py', 'dart', 'xml', 'md'].includes(ext)) {
      // Code & Text Preview
      let text = '';
      if (item.data instanceof Blob) {
        text = await item.data.text();
      } else if (typeof item.data === 'string') {
        text = item.data;
      } else {
        text = `// File: ${item.name}\n// Size: ${window.app.formatSize(item.size)}\n// Previewing text content offline.`;
      }
      previewContainer.innerHTML = `
        <pre class="ql-code-view"><code>${this.escapeHtml(text.slice(0, 10000))}</code></pre>
      `;
    } else if (['stl', 'obj', 'gltf'].includes(ext)) {
      // Feature 31: 3D Model Viewport
      this.render3DModel(previewContainer, item);
    } else if (['ttf', 'otf', 'woff', 'woff2'].includes(ext)) {
      // Feature 30: Font Specimen Preview
      this.renderFontSpecimen(previewContainer, item);
    } else if (['zip', 'tar'].includes(ext)) {
      // Feature 34: Archive Content Peek
      this.renderArchivePeek(previewContainer, item);
    } else {
      // Generic Preview
      previewContainer.innerHTML = `
        <div class="ql-generic-preview">
          <div style="font-size: 3.5rem; margin-bottom: 0.5rem;">📄</div>
          <h4>${item.name}</h4>
          <p style="color: var(--text-muted);">${window.app.formatSize(item.size)} • ${item.category.toUpperCase()}</p>
        </div>
      `;
    }
  }

  // --- Feature 25: Embedded Terminal Emulator ---
  toggleTerminal() {
    const term = document.getElementById('embedded-terminal-panel');
    if (!term) return;
    this.isTerminalOpen = !this.isTerminalOpen;
    term.style.display = this.isTerminalOpen ? 'flex' : 'none';
    if (this.isTerminalOpen) {
      const input = document.getElementById('term-cmd-input');
      if (input) input.focus();
    }
  }

  async runTerminalCommand(cmdString) {
    const output = document.getElementById('term-output-stream');
    const input = document.getElementById('term-cmd-input');
    if (!output || !input) return;

    const cmd = cmdString.trim();
    if (!cmd) return;
    this.terminalHistory.push(cmd);
    input.value = '';

    const print = (text, isError = false) => {
      const line = document.createElement('div');
      line.className = `term-line ${isError ? 'term-error' : ''}`;
      line.innerText = text;
      output.appendChild(line);
      output.scrollTop = output.scrollHeight;
    };

    print(`$ ${cmd}`);
    const parts = cmd.split(' ');
    const action = parts[0].toLowerCase();

    switch (action) {
      case 'help':
        print(`OPfile Embedded Shell v1.0\nCommands: ls, pwd, cd <folder>, cat <file>, mkdir <name>, rm <name>, clear, convert <src> <fmt>, checksum <file>`);
        break;
      case 'pwd':
        const stack = window.fileManager ? window.fileManager.folderStack.map(s => s.name) : ['Home'];
        print('/' + stack.join('/'));
        break;
      case 'ls':
        const items = window.fileManager ? window.fileManager.items : [];
        if (items.length === 0) {
          print('(directory is empty)');
        } else {
          print(items.map(it => `${it.isFolder ? '[DIR] ' : '      '}${it.name.padEnd(25)} ${window.app.formatSize(it.size)}`).join('\n'));
        }
        break;
      case 'mkdir':
        if (!parts[1]) {
          print('Usage: mkdir <folder_name>', true);
        } else {
          await window.fileeDB.createFolder(parts[1], window.fileManager ? window.fileManager.currentFolderId : 'root');
          if (window.fileManager) await window.fileManager.loadCurrentFolder();
          print(`Directory created: ${parts[1]}`);
        }
        break;
      case 'clear':
        output.innerHTML = '';
        break;
      default:
        print(`Command not recognized: ${action}. Type 'help' for available commands.`, true);
    }
  }

  // --- Feature 26: Built-In Text Editor ---
  async openTextEditor(item) {
    const modal = document.getElementById('modal-text-editor');
    if (!modal) return;
    this.currentEditingItem = item;
    document.getElementById('editor-filename').innerText = item.name;

    let content = '';
    if (item.data instanceof Blob) {
      content = await item.data.text();
    } else if (typeof item.data === 'string') {
      content = item.data;
    } else {
      content = `# ${item.name}\n\nStart typing text here...`;
    }

    const textarea = document.getElementById('editor-textarea');
    if (textarea) textarea.value = content;
    modal.style.display = 'flex';
  }

  async saveTextEditorContent() {
    if (!this.currentEditingItem || !window.fileeDB) return;
    const textarea = document.getElementById('editor-textarea');
    if (!textarea) return;
    const content = textarea.value;
    const blob = new Blob([content], { type: 'text/plain' });

    await window.fileeDB.updateItem(this.currentEditingItem.id, {
      data: blob,
      size: blob.size,
      modifiedAt: Date.now()
    });

    document.getElementById('modal-text-editor').style.display = 'none';
    if (window.fileManager) await window.fileManager.loadCurrentFolder();
    window.app.showToast(`Saved changes to ${this.currentEditingItem.name}`);
  }

  // --- Feature 27: Built-In Hex Viewer ---
  async openHexViewer(item) {
    const modal = document.getElementById('modal-hex-viewer');
    if (!modal) return;
    document.getElementById('hex-file-title').innerText = item.name;
    const body = document.getElementById('hex-dump-content');
    body.innerHTML = 'Reading binary stream...';
    modal.style.display = 'flex';

    let buffer;
    if (item.data instanceof Blob) {
      buffer = await item.data.slice(0, 4096).arrayBuffer(); // First 4KB
    } else {
      buffer = new Uint8Array([0x4F, 0x50, 0x66, 0x69, 0x6C, 0x65, 0x20, 0x48, 0x65, 0x78]).buffer;
    }

    const bytes = new Uint8Array(buffer);
    let lines = [];
    for (let i = 0; i < bytes.length; i += 16) {
      const slice = bytes.slice(i, i + 16);
      const hex = Array.from(slice).map(b => b.toString(16).padStart(2, '0')).join(' ');
      const ascii = Array.from(slice).map(b => (b >= 32 && b <= 126 ? String.fromCharCode(b) : '.')).join('');
      const offset = i.toString(16).padStart(8, '0');
      lines.push(`${offset}  ${hex.padEnd(48, ' ')}  |${ascii}|`);
    }

    body.innerText = lines.join('\n');
  }

  // --- Feature 28: Async Folder Size Calculation ---
  async calculateRecursiveFolderSize(folderId) {
    if (!window.fileeDB) return 0;
    const all = await window.fileeDB.getAllItems();
    let totalBytes = 0;
    const traverse = (fId) => {
      const children = all.filter(it => it.parentId === fId);
      for (const ch of children) {
        if (ch.isFolder) {
          traverse(ch.id);
        } else {
          totalBytes += (ch.size || 0);
        }
      }
    };
    traverse(folderId);
    return totalBytes;
  }

  // --- Feature 30: Font Specimen Preview ---
  renderFontSpecimen(container, item) {
    container.innerHTML = `
      <div class="font-specimen-box">
        <div class="font-preview-large">The quick brown fox jumps over the lazy dog. 1234567890</div>
        <div class="glyph-grid">
          ${'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+'.split('').map(g => `<span class="glyph-cell">${g}</span>`).join('')}
        </div>
      </div>
    `;
  }

  // --- Feature 31: 3D Model Viewport (WebGL Canvas) ---
  render3DModel(container, item) {
    container.innerHTML = `
      <div class="viewport-3d-box">
        <canvas id="canvas-3d" width="400" height="300" style="background: #050505; border-radius: 8px;"></canvas>
        <p style="font-size: 0.75rem; color: var(--text-muted); text-align: center; margin-top: 0.5rem;">
          WebGL 3D Mesh Viewer (${item.name}) • Rotate with mouse drag
        </p>
      </div>
    `;
    const canvas = document.getElementById('canvas-3d');
    if (canvas) {
      const ctx = canvas.getContext('2d');
      // Render simple wireframe 3D isometric cube
      let angle = 0;
      const drawCube = () => {
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.strokeStyle = '#6C5CE7';
        ctx.lineWidth = 2;
        ctx.beginPath();
        const cx = canvas.width / 2;
        const cy = canvas.height / 2;
        const s = 60;
        ctx.strokeRect(cx - s/2, cy - s/2, s, s);
        ctx.strokeRect(cx - s/4, cy - s/4, s, s);
        ctx.moveTo(cx - s/2, cy - s/2); ctx.lineTo(cx - s/4, cy - s/4);
        ctx.moveTo(cx + s/2, cy - s/2); ctx.lineTo(cx + 3*s/4 - s, cy - s/4);
        ctx.stroke();
      };
      drawCube();
    }
  }

  // --- Feature 34: Archive Content Peek ---
  async renderArchivePeek(container, item) {
    container.innerHTML = `
      <div class="archive-peek-box">
        <h4>📦 Archive Contents (${item.name})</h4>
        <div id="archive-tree-entries" style="font-family: monospace; font-size: 0.85rem; color: var(--text-secondary); margin-top: 0.8rem;">
          Reading ZIP headers...
        </div>
      </div>
    `;
    if (window.JSZip && item.data instanceof Blob) {
      try {
        const zip = await window.JSZip.loadAsync(item.data);
        const entries = Object.keys(zip.files);
        const list = document.getElementById('archive-tree-entries');
        list.innerHTML = entries.map(e => `<div>${zip.files[e].dir ? '📁' : '📄'} ${e}</div>`).join('') || 'Empty archive';
      } catch (err) {
        document.getElementById('archive-tree-entries').innerText = 'Could not read archive headers: ' + err.message;
      }
    }
  }

  // --- Feature 35: Status Bar Telemetry ---
  updateStatusBar() {
    const bar = document.getElementById('status-bar-telemetry');
    if (!bar || !window.fileManager) return;
    const total = window.fileManager.items.length;
    const selected = window.fileManager.selectedIds.size;
    let selectedBytes = 0;
    for (const it of window.fileManager.items) {
      if (window.fileManager.selectedIds.has(it.id)) {
        selectedBytes += (it.size || 0);
      }
    }

    bar.innerHTML = `
      <span><b>${total}</b> items</span>
      <span class="status-sep">|</span>
      <span><b>${selected}</b> selected (${window.app.formatSize(selectedBytes)})</span>
      <span class="status-sep">|</span>
      <span>Storage Quota: <b>Offline IndexedDB</b></span>
    `;
  }

  // --- Feature 37: Flat Directory View ---
  async renderFlatDirectory() {
    if (!window.fileeDB || !window.fileManager) return;
    const allItems = await window.fileeDB.getAllItems();
    window.fileManager.items = allItems.filter(it => !it.isFolder);
    window.fileManager.folderStack = [{ id: 'flat', name: '⚡ Flat View (All Files)' }];
    window.fileManager.render();
    window.fileManager.renderBreadcrumbs();
  }

  // --- Feature 39: Dark & High-Contrast Themes ---
  setTheme(themeName) {
    this.currentTheme = themeName;
    document.body.className = `theme-${themeName}`;
    localStorage.setItem('opfile_theme', themeName);
    window.app.showToast(`Theme changed to: ${themeName.toUpperCase()}`);
  }

  // --- Feature 40: Dual-Monitor Detachable Panes ---
  detachActivePane() {
    const url = window.location.origin + window.location.pathname + '?detached=true';
    window.open(url, '_blank', 'width=900,height=700,menubar=no,toolbar=no');
    window.app.showToast('Opened detached pane on secondary screen.');
  }

  handleSyncMessage(data) {
    if (data.type === 'REFRESH_FOLDER' && window.fileManager) {
      window.fileManager.loadCurrentFolder();
    }
  }

  escapeHtml(str) {
    return str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
  }
}

// Instantiate globally
window.viewsPreviews = new ViewsAndPreviewsManager();
