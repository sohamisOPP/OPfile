/**
 * Filee Mobile - Main Application Controller
 * Handles Navigation, Modals, Donut Converter Flow, Previews, and Settings.
 */

class FileeApp {
  constructor() {
    this.currentTab = 'files';
    this.activeConvertingItem = null;
    this.activeTargetFormat = null;
    this.donutInstance = null;
    this.deferredPrompt = null;
  }

  async init() {
    // 1. Initialize IndexedDB
    await window.fileeDB.init();

    // 2. Initialize File Manager
    await window.fileManager.init();

    // 3. Setup Navigation & Tabs
    this.initTabs();

    // 4. Setup Global Event Listeners (Upload, Search, Modals)
    this.initEventListeners();

    // 5. Setup Donut Converter Canvas
    this.initDonut();

    // 6. Setup Service Worker for 100% Offline PWA
    this.initServiceWorker();

    // 7. Render Initial Storage & History
    await this.updateStorageInfo();
    await this.loadRecentHistory();

    // 8. Initialize Features 1-200 Extensions
    if (window.coreNav) {
      window.coreNav.initFunctionKeys();
      window.coreNav.initMouseGestures();
      window.coreNav.renderPanes();
      window.coreNav.renderTabs(0);
      window.coreNav.renderBreadcrumbs(window.fileManager.folderStack);
    }
    if (window.viewsPreviews) {
      window.viewsPreviews.updateStatusBar();
    }

    console.log('OPfile Initialized Successfully.');
  }

  initTabs() {
    const tabButtons = document.querySelectorAll('.nav-tab-btn');
    tabButtons.forEach((btn) => {
      btn.onclick = () => {
        const tab = btn.dataset.tab;
        this.switchTab(tab);
      };
    });
  }

  switchTab(tabName) {
    this.currentTab = tabName;

    // Update Tab Buttons
    document.querySelectorAll('.nav-tab-btn').forEach((btn) => {
      btn.classList.toggle('active', btn.dataset.tab === tabName);
    });

    // Update Tab Views
    document.querySelectorAll('.app-view').forEach((view) => {
      view.classList.toggle('active', view.id === `view-${tabName}`);
    });

    // Refresh contents if needed
    if (tabName === 'files') {
      window.fileManager.loadCurrentFolder();
    } else if (tabName === 'recent') {
      this.loadRecentHistory();
    } else if (tabName === 'settings') {
      this.updateStorageInfo();
    }
  }

  initDonut() {
    const canvas = document.getElementById('donut-canvas');
    if (canvas) {
      this.donutInstance = new window.FileeDonut(canvas);
    }
  }

  initEventListeners() {
    // Search input
    const searchInput = document.getElementById('search-input');
    if (searchInput) {
      let debounceTimer;
      searchInput.oninput = (e) => {
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(() => {
          window.fileManager.searchQuery = e.target.value;
          window.fileManager.loadCurrentFolder();
        }, 200);
      };
    }

    // Category filter pills
    const filterPills = document.querySelectorAll('.filter-pill');
    filterPills.forEach((pill) => {
      pill.onclick = () => {
        filterPills.forEach((p) => p.classList.remove('active'));
        pill.classList.add('active');
        window.fileManager.filterCategory = pill.dataset.category;
        window.fileManager.loadCurrentFolder();
      };
    });

    // View Mode Toggle (Grid / List)
    const viewToggleBtn = document.getElementById('btn-toggle-view');
    if (viewToggleBtn) {
      viewToggleBtn.onclick = async () => {
        const newMode = window.fileManager.viewMode === 'grid' ? 'list' : 'grid';
        window.fileManager.viewMode = newMode;
        await window.fileeDB.setSetting('viewMode', newMode);
        window.fileManager.render();
      };
    }

    // Floating Action Button (FAB) / Add Button
    const btnFab = document.getElementById('btn-fab-add');
    const inputFileInput = document.getElementById('hidden-file-input');
    if (btnFab && inputFileInput) {
      btnFab.onclick = () => {
        this.showAddMenu();
      };
      inputFileInput.onchange = (e) => {
        window.fileManager.importFiles(e.target.files);
        inputFileInput.value = '';
      };
    }

    // Multi-Selection Actions
    const btnBatchDelete = document.getElementById('btn-batch-delete');
    if (btnBatchDelete) {
      btnBatchDelete.onclick = async () => {
        if (confirm(`Delete ${window.fileManager.selectedIds.size} selected items?`)) {
          for (const id of window.fileManager.selectedIds) {
            await window.fileeDB.deleteItem(id);
          }
          window.fileManager.clearSelection();
          await window.fileManager.loadCurrentFolder();
          this.showToast('Selected items deleted', 'info');
        }
      };
    }

    const btnBatchZip = document.getElementById('btn-batch-zip');
    if (btnBatchZip) {
      btnBatchZip.onclick = async () => {
        this.batchCompressToZip();
      };
    }

    const btnClearSelection = document.getElementById('btn-clear-selection');
    if (btnClearSelection) {
      btnClearSelection.onclick = () => window.fileManager.clearSelection();
    }

    // Close Modals on Backdrop Click
    document.querySelectorAll('.modal-overlay').forEach((overlay) => {
      overlay.onclick = (e) => {
        if (e.target === overlay) {
          overlay.classList.remove('active');
        }
      };
    });

    // PWA Install prompt listener
    window.addEventListener('beforeinstallprompt', (e) => {
      e.preventDefault();
      this.deferredPrompt = e;
      const banner = document.getElementById('pwa-install-banner');
      if (banner) banner.style.display = 'flex';
    });

    const btnInstallPwa = document.getElementById('btn-install-pwa');
    if (btnInstallPwa) {
      btnInstallPwa.onclick = async () => {
        if (this.deferredPrompt) {
          this.deferredPrompt.prompt();
          const { outcome } = await this.deferredPrompt.userChoice;
          console.log('PWA install outcome:', outcome);
          this.deferredPrompt = null;
          document.getElementById('pwa-install-banner').style.display = 'none';
        }
      };
    }
  }

  // --- Add / Import Menu ---
  showAddMenu() {
    const sheet = document.getElementById('action-sheet-add');
    if (sheet) sheet.classList.add('active');
  }

  hideAddMenu() {
    const sheet = document.getElementById('action-sheet-add');
    if (sheet) sheet.classList.remove('active');
  }

  promptNewFolder() {
    this.hideAddMenu();
    const name = prompt('Folder Name:', 'New Folder');
    if (name) {
      window.fileManager.createFolder(name);
    }
  }

  triggerDeviceUpload() {
    this.hideAddMenu();
    const input = document.getElementById('hidden-file-input');
    if (input) input.click();
  }

  // --- Donut Converter Modal Flow ---
  async openDonutConverter(item) {
    this.activeConvertingItem = item;
    const modal = document.getElementById('modal-donut-converter');
    const fileNameEl = document.getElementById('donut-file-name');
    const fileMetaEl = document.getElementById('donut-file-meta');
    const optionsPanel = document.getElementById('donut-options-panel');
    const btnExecute = document.getElementById('btn-execute-conversion');

    if (!modal) return;

    // Reset UI state
    optionsPanel.style.display = 'none';
    btnExecute.style.display = 'none';
    fileNameEl.textContent = item.name;
    fileMetaEl.textContent = `${window.fileManager.formatFileSize(item.size || (item.data ? item.data.size : 0))} • ${item.category.toUpperCase()}`;

    // Get compatible formats
    const ext = window.fileeConverter.getExtension(item.name);
    const targetFormats = window.fileeConverter.getTargetFormats(ext);

    // Setup Donut
    this.donutInstance.setItems(
      targetFormats,
      (selectedFormat) => {
        this.selectTargetFormat(selectedFormat);
      },
      () => {
        this.closeDonutConverter();
      }
    );

    // Open Modal
    modal.classList.add('active');
  }

  selectTargetFormat(targetFormat) {
    this.activeTargetFormat = targetFormat;
    const optionsPanel = document.getElementById('donut-options-panel');
    const optionsContainer = document.getElementById('converter-options-fields');
    const selectedBadge = document.getElementById('selected-target-badge');
    const btnExecute = document.getElementById('btn-execute-conversion');

    selectedBadge.textContent = `Converting to: ${targetFormat.name} (.${targetFormat.id})`;
    optionsPanel.style.display = 'block';
    btnExecute.style.display = 'flex';
    optionsContainer.innerHTML = '';

    // Render Options depending on target category
    if (targetFormat.category === 'image') {
      optionsContainer.innerHTML = `
        <div class="option-row">
          <label>Quality (${targetFormat.id.toUpperCase()})</label>
          <div class="slider-container">
            <input type="range" id="opt-quality" min="10" max="100" value="90" oninput="document.getElementById('val-quality').textContent = this.value + '%'">
            <span id="val-quality" class="slider-val">90%</span>
          </div>
        </div>
        <div class="option-row">
          <label>Resize Max Dimension (px)</label>
          <input type="number" id="opt-dimension" placeholder="Original size" class="input-text">
        </div>
        <div class="option-row-toggle">
          <label>Grayscale (Black & White)</label>
          <input type="checkbox" id="opt-grayscale" class="toggle-checkbox">
        </div>
        <div class="option-row">
          <label>Rotate</label>
          <select id="opt-rotation" class="input-select">
            <option value="0">0° (No change)</option>
            <option value="90">90° Clockwise</option>
            <option value="180">180° Half turn</option>
            <option value="270">270° Counter-clockwise</option>
          </select>
        </div>
      `;
    } else if (targetFormat.category === 'pdf') {
      optionsContainer.innerHTML = `
        <div class="option-row-toggle">
          <label>Fit to standard A4 page</label>
          <input type="checkbox" id="opt-fit-a4" checked class="toggle-checkbox">
        </div>
      `;
    } else if (targetFormat.category === 'archive') {
      optionsContainer.innerHTML = `
        <div class="option-row">
          <label>Compression Level</label>
          <select id="opt-zip-level" class="input-select">
            <option value="1">Fast (Low compression)</option>
            <option value="6" selected>Balanced (Standard)</option>
            <option value="9">Maximum (Smallest size)</option>
          </select>
        </div>
      `;
    } else {
      optionsContainer.innerHTML = `
        <div class="option-info">Ready to convert immediately. 100% offline.</div>
      `;
    }

    // Scroll down to options
    optionsPanel.scrollIntoView({ behavior: 'smooth' });
  }

  async executeConversion() {
    if (!this.activeConvertingItem || !this.activeTargetFormat) return;

    const item = this.activeConvertingItem;
    const target = this.activeTargetFormat;
    const progressEl = document.getElementById('conversion-progress-box');
    const progressBar = document.getElementById('conversion-progress-bar');
    const statusText = document.getElementById('conversion-status-text');

    progressEl.style.display = 'block';
    progressBar.style.width = '30%';
    statusText.textContent = `Processing ${item.name}...`;

    try {
      const srcBlob = item.data;
      const srcExt = window.fileeConverter.getExtension(item.name);
      let resultBlob = null;
      let finalExt = target.id;

      // --- 1. Image Conversions ---
      if (['png', 'jpg', 'jpeg', 'webp', 'bmp', 'ico'].includes(target.id) && item.category === 'image') {
        const qualityVal = document.getElementById('opt-quality') ? parseInt(document.getElementById('opt-quality').value) / 100 : 0.9;
        const maxDim = document.getElementById('opt-dimension') && document.getElementById('opt-dimension').value ? parseInt(document.getElementById('opt-dimension').value) : null;
        const grayscale = document.getElementById('opt-grayscale') ? document.getElementById('opt-grayscale').checked : false;
        const rotation = document.getElementById('opt-rotation') ? parseInt(document.getElementById('opt-rotation').value) : 0;

        progressBar.style.width = '70%';
        const converted = await window.fileeConverter.convertImage(srcBlob, {
          targetFormat: target.id,
          quality: qualityVal,
          width: maxDim,
          grayscale,
          rotation
        });
        resultBlob = converted.blob;
        finalExt = converted.ext;
      }
      // --- 2. Image to PDF ---
      else if (target.id === 'pdf' && item.category === 'image') {
        progressBar.style.width = '70%';
        const fitA4 = document.getElementById('opt-fit-a4') ? document.getElementById('opt-fit-a4').checked : true;
        resultBlob = await window.fileeConverter.imagesToPdf([{ blob: srcBlob }], { fitToPage: fitA4 });
        finalExt = 'pdf';
      }
      // --- 3. PDF to Images ---
      else if (['png', 'jpg'].includes(target.id) && srcExt === 'pdf') {
        progressBar.style.width = '60%';
        statusText.textContent = 'Rendering PDF pages...';
        const pages = await window.fileeConverter.pdfToImages(srcBlob, { format: target.id });

        if (pages.length === 1) {
          resultBlob = pages[0].blob;
          finalExt = target.id;
        } else {
          // If multiple pages, package in a ZIP
          statusText.textContent = 'Packaging pages in ZIP archive...';
          const zipFiles = pages.map((p) => ({ name: p.filename, data: p.blob }));
          resultBlob = await window.fileeConverter.createZip(zipFiles);
          finalExt = 'zip';
        }
      }
      // --- 4. PDF to Text ---
      else if (target.id === 'txt' && srcExt === 'pdf') {
        progressBar.style.width = '70%';
        resultBlob = await window.fileeConverter.extractPdfText(srcBlob);
        finalExt = 'txt';
      }
      // --- 5. Media to WAV ---
      else if (target.id === 'wav') {
        progressBar.style.width = '65%';
        statusText.textContent = 'Extracting and encoding WAV audio...';
        resultBlob = await window.fileeConverter.convertToWav(srcBlob);
        finalExt = 'wav';
      }
      // --- 6. Video Snapshot ---
      else if (['jpg', 'png'].includes(target.id) && item.category === 'video') {
        progressBar.style.width = '70%';
        resultBlob = await window.fileeConverter.extractVideoFrame(srcBlob, 1.0, target.id);
        finalExt = target.id;
      }
      // --- 7. Compress into ZIP ---
      else if (target.id === 'zip') {
        progressBar.style.width = '70%';
        const level = document.getElementById('opt-zip-level') ? parseInt(document.getElementById('opt-zip-level').value) : 6;
        resultBlob = await window.fileeConverter.createZip([{ name: item.name, data: srcBlob }], { level });
        finalExt = 'zip';
      }
      // --- 8. Extract ZIP ---
      else if (target.id === 'extract' && srcExt === 'zip') {
        progressBar.style.width = '70%';
        statusText.textContent = 'Extracting archive entries...';
        const entries = await window.fileeConverter.extractZip(srcBlob);
        for (const entry of entries) {
          const entryFormat = window.fileeConverter.getFormat(entry.name, entry.blob.type);
          await window.fileeDB.saveItem({
            name: entry.name,
            isFolder: false,
            parentId: window.fileManager.currentFolderId,
            size: entry.size,
            type: entry.blob.type || entryFormat.mime,
            category: entryFormat.category,
            data: entry.blob,
            modifiedAt: Date.now()
          });
        }
        await window.fileManager.loadCurrentFolder();
        this.closeDonutConverter();
        this.showToast(`Extracted ${entries.length} files successfully!`, 'success');
        return;
      }
      // --- 9. Data Conversions ---
      else if (srcExt === 'json' && target.id === 'csv') {
        const text = await srcBlob.text();
        resultBlob = window.fileeConverter.jsonToCsv(text);
        finalExt = 'csv';
      } else if (srcExt === 'csv' && target.id === 'json') {
        const text = await srcBlob.text();
        resultBlob = window.fileeConverter.csvToJson(text);
        finalExt = 'json';
      } else if (srcExt === 'md' && target.id === 'html') {
        const text = await srcBlob.text();
        resultBlob = window.fileeConverter.markdownToHtml(text);
        finalExt = 'html';
      } else {
        throw new Error(`Conversion path from ${srcExt} to ${target.id} is not supported directly.`);
      }

      progressBar.style.width = '100%';
      statusText.textContent = 'Conversion complete!';

      // Save Output into IndexedDB
      const savedItem = await window.fileManager.saveConvertedFile(resultBlob, item.name, finalExt);

      // Add to History
      await window.fileeDB.addHistory({
        sourceName: item.name,
        targetName: savedItem.name,
        sourceFormat: srcExt,
        targetFormat: finalExt,
        sizeBefore: item.size || item.data.size,
        sizeAfter: resultBlob.size
      });

      // Show Success Toast
      this.closeDonutConverter();
      this.showToast(`Converted: ${savedItem.name}`, 'success');

      // Automatically offer download or preview
      this.showDownloadPrompt(savedItem);
    } catch (err) {
      console.error('Conversion failed:', err);
      alert(`Conversion Error: ${err.message}`);
    } finally {
      progressEl.style.display = 'none';
    }
  }

  closeDonutConverter() {
    const modal = document.getElementById('modal-donut-converter');
    if (modal) modal.classList.remove('active');
    this.activeConvertingItem = null;
    this.activeTargetFormat = null;
  }

  showDownloadPrompt(savedItem) {
    if (confirm(`Conversion finished!\nWould you like to download "${savedItem.name}" to your phone now?`)) {
      this.downloadFile(savedItem);
    }
  }

  // --- File Actions & Preview ---
  showFileActionSheet(item) {
    const sheet = document.getElementById('action-sheet-file');
    const title = document.getElementById('sheet-file-title');
    if (!sheet) return;

    title.textContent = item.name;

    document.getElementById('sheet-btn-convert').onclick = () => {
      this.hideFileActionSheet();
      this.openDonutConverter(item);
    };

    document.getElementById('sheet-btn-preview').onclick = () => {
      this.hideFileActionSheet();
      this.previewFile(item);
    };

    document.getElementById('sheet-btn-download').onclick = () => {
      this.hideFileActionSheet();
      this.downloadFile(item);
    };

    document.getElementById('sheet-btn-rename').onclick = () => {
      this.hideFileActionSheet();
      const newName = prompt('New file name:', item.name);
      if (newName && newName !== item.name) {
        window.fileManager.renameItem(item.id, newName);
      }
    };

    document.getElementById('sheet-btn-delete').onclick = () => {
      this.hideFileActionSheet();
      if (confirm(`Delete "${item.name}"?`)) {
        window.fileManager.deleteItem(item.id);
      }
    };

    sheet.classList.add('active');
  }

  hideFileActionSheet() {
    const sheet = document.getElementById('action-sheet-file');
    if (sheet) sheet.classList.remove('active');
  }

  async previewFile(item) {
    const modal = document.getElementById('modal-preview');
    const title = document.getElementById('preview-title');
    const body = document.getElementById('preview-body');
    if (!modal) return;

    title.textContent = item.name;
    body.innerHTML = '<div class="loading-spinner">Loading preview...</div>';
    modal.classList.add('active');

    const cat = item.category || 'other';
    const blob = item.data;
    const url = URL.createObjectURL(blob);

    try {
      if (cat === 'image') {
        body.innerHTML = `<img src="${url}" class="preview-media-image" alt="${item.name}" />`;
      } else if (cat === 'audio') {
        body.innerHTML = `
          <div class="preview-audio-player">
            <div class="audio-icon">🎵</div>
            <audio controls src="${url}" autoplay style="width: 100%; margin-top: 1rem;"></audio>
          </div>
        `;
      } else if (cat === 'video') {
        body.innerHTML = `
          <video controls playsinline src="${url}" style="max-width: 100%; max-height: 70vh; border-radius: 8px;"></video>
        `;
      } else if (cat === 'pdf') {
        body.innerHTML = `
          <iframe src="${url}" class="preview-pdf-frame"></iframe>
        `;
      } else if (cat === 'document' || cat === 'data' || ['txt', 'md', 'json', 'csv', 'xml', 'html'].includes(window.fileeConverter.getExtension(item.name))) {
        const text = await blob.text();
        const escaped = text.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
        body.innerHTML = `
          <pre class="preview-text-content"><code>${escaped}</code></pre>
        `;
      } else {
        body.innerHTML = `
          <div class="preview-fallback">
            <div class="fallback-icon">📄</div>
            <p>Preview not available for this format.</p>
            <button class="btn btn-primary" onclick="window.app.downloadFile(window.fileManager.items.find(i=>i.id==='${item.id}'))">Download to View</button>
          </div>
        `;
      }
    } catch (err) {
      body.innerHTML = `<div class="preview-error">Error loading preview: ${err.message}</div>`;
    }
  }

  closePreview() {
    const modal = document.getElementById('modal-preview');
    if (modal) modal.classList.remove('active');
  }

  downloadFile(item) {
    if (!item || !item.data) return;
    const a = document.createElement('a');
    const url = URL.createObjectURL(item.data);
    a.href = url;
    a.download = item.name;
    document.body.appendChild(a);
    a.click();
    setTimeout(() => {
      document.body.removeChild(a);
      URL.revokeObjectURL(url);
    }, 100);
    this.showToast(`Downloading "${item.name}"...`, 'info');
  }

  // --- Batch Compress ---
  async batchCompressToZip() {
    const ids = Array.from(window.fileManager.selectedIds);
    if (ids.length === 0) return;

    const filesToZip = [];
    for (const id of ids) {
      const item = await window.fileeDB.getItem(id);
      if (item && !item.isFolder && item.data) {
        filesToZip.push({ name: item.name, data: item.data });
      }
    }

    if (filesToZip.length === 0) {
      alert('Please select files (not empty folders) to compress.');
      return;
    }

    this.showToast('Compressing files into ZIP...', 'info');
    const zipBlob = await window.fileeConverter.createZip(filesToZip);
    const zipName = `archive_${Date.now()}.zip`;

    const saved = await window.fileeDB.saveItem({
      name: zipName,
      isFolder: false,
      parentId: window.fileManager.currentFolderId,
      size: zipBlob.size,
      type: 'application/zip',
      category: 'archive',
      data: zipBlob,
      modifiedAt: Date.now()
    });

    window.fileManager.clearSelection();
    await window.fileManager.loadCurrentFolder();
    this.showToast(`Created ${zipName}`, 'success');
  }

  // --- Recent History ---
  async loadRecentHistory() {
    const container = document.getElementById('recent-history-list');
    if (!container) return;

    const history = await window.fileeDB.getHistory(30);
    if (history.length === 0) {
      container.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">🕒</div>
          <h3>No conversions yet</h3>
          <p>Files converted with Filee will appear here.</p>
        </div>
      `;
      return;
    }

    container.innerHTML = '';
    history.forEach((h) => {
      const item = document.createElement('div');
      item.className = 'history-item';
      const ratio = ((1 - h.sizeAfter / h.sizeBefore) * 100).toFixed(0);
      const ratioText = ratio > 0 ? `-${ratio}%` : `+${Math.abs(ratio)}%`;
      const dateStr = window.fileManager.formatDate(h.timestamp);

      item.innerHTML = `
        <div class="history-badge">🔄</div>
        <div class="history-details">
          <div class="history-title">${h.sourceName} ➔ <b>${h.targetName}</b></div>
          <div class="history-meta">
            <span>${window.fileManager.formatFileSize(h.sizeBefore)} ➔ ${window.fileManager.formatFileSize(h.sizeAfter)} (${ratioText})</span>
            <span class="meta-dot">•</span>
            <span>${dateStr}</span>
          </div>
        </div>
      `;
      container.appendChild(item);
    });
  }

  async clearRecentHistory() {
    if (confirm('Clear all conversion history?')) {
      await window.fileeDB.clearHistory();
      await this.loadRecentHistory();
      this.showToast('History cleared', 'info');
    }
  }

  // --- Settings & Storage ---
  async updateStorageInfo() {
    const storageEl = document.getElementById('storage-bar-fill');
    const storageText = document.getElementById('storage-used-text');
    const fileCountText = document.getElementById('storage-file-count');

    if (!storageEl || !storageText) return;

    const stats = await window.fileeDB.getStorageStats();
    const formatted = window.fileManager.formatFileSize(stats.totalBytes);

    storageText.textContent = formatted;
    if (fileCountText) fileCountText.textContent = `${stats.fileCount} files stored locally`;

    // Visual percentage out of 500 MB quota
    const percent = Math.min(100, Math.max(3, (stats.totalBytes / (500 * 1024 * 1024)) * 100));
    storageEl.style.width = `${percent}%`;
  }

  // --- Toast Notifications ---
  showToast(message, type = 'info') {
    const toast = document.getElementById('app-toast');
    if (!toast) return;

    toast.textContent = message;
    toast.className = `toast visible toast-${type}`;
    clearTimeout(this.toastTimer);
    this.toastTimer = setTimeout(() => {
      toast.classList.remove('visible');
    }, 3000);
  }

  // --- Service Worker Registration ---
  initServiceWorker() {
    if ('serviceWorker' in navigator) {
      navigator.serviceWorker
        .register('/sw.js')
        .then((reg) => console.log('Service Worker registered for offline use:', reg.scope))
        .catch((err) => console.warn('Service worker registration failed:', err));
    }
  }
}

window.app = new FileeApp();

// Boot application when DOM is ready
document.addEventListener('DOMContentLoaded', () => {
  window.app.init();
});
