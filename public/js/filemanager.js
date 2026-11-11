/**
 * Filee Mobile - File Manager Module
 * Fast, responsive mobile file browser with persistent storage, preview, and batch operations.
 */

class FileManager {
  constructor() {
    this.currentFolderId = 'root';
    this.folderStack = [{ id: 'root', name: 'Home' }];
    this.items = [];
    this.selectedIds = new Set();
    this.viewMode = 'grid'; // 'grid' | 'list'
    this.filterCategory = 'all'; // 'all' | 'image' | 'document' | 'audio' | 'video' | 'archive'
    this.searchQuery = '';
    this.isSelectionMode = false;

    this.container = document.getElementById('file-grid-container');
    this.breadcrumbsContainer = document.getElementById('breadcrumbs-bar');
  }

  async init() {
    this.viewMode = (await window.fileeDB.getSetting('viewMode')) || 'grid';
    await this.loadCurrentFolder();
  }

  async loadCurrentFolder() {
    this.selectedIds.clear();
    this.updateSelectionBar();

    let items;
    if (this.searchQuery) {
      items = await window.fileeDB.searchFiles(this.searchQuery);
    } else {
      items = await window.fileeDB.getItemsInFolder(this.currentFolderId);
    }

    // Apply Category Filter
    if (this.filterCategory !== 'all') {
      items = items.filter((item) => item.isFolder || item.category === this.filterCategory);
    }

    this.items = items;
    this.render();
    this.renderBreadcrumbs();
  }

  async navigateToFolder(folderId, folderName) {
    this.searchQuery = '';
    const searchInput = document.getElementById('search-input');
    if (searchInput) searchInput.value = '';

    this.currentFolderId = folderId;
    this.folderStack.push({ id: folderId, name: folderName });
    await this.loadCurrentFolder();
  }

  async navigateBack() {
    if (this.folderStack.length > 1) {
      this.folderStack.pop();
      const prev = this.folderStack[this.folderStack.length - 1];
      this.currentFolderId = prev.id;
      await this.loadCurrentFolder();
    }
  }

  async navigateToBreadcrumb(index) {
    if (index >= 0 && index < this.folderStack.length) {
      this.folderStack = this.folderStack.slice(0, index + 1);
      const target = this.folderStack[index];
      this.currentFolderId = target.id;
      await this.loadCurrentFolder();
    }
  }

  renderBreadcrumbs() {
    if (window.coreNav) {
      window.coreNav.renderBreadcrumbs(this.folderStack);
      return;
    }
    if (!this.breadcrumbsContainer) return;
    this.breadcrumbsContainer.innerHTML = '';

    this.folderStack.forEach((crumb, idx) => {
      const isLast = idx === this.folderStack.length - 1;
      const span = document.createElement('span');
      span.className = `breadcrumb-item ${isLast ? 'active' : ''}`;
      span.textContent = crumb.name;
      if (!isLast) {
        span.onclick = () => this.navigateToBreadcrumb(idx);
      }
      this.breadcrumbsContainer.appendChild(span);

      if (!isLast) {
        const sep = document.createElement('span');
        sep.className = 'breadcrumb-separator';
        sep.textContent = '/';
        this.breadcrumbsContainer.appendChild(sep);
      }
    });
  }

  render() {
    if (!this.container) return;
    this.container.innerHTML = '';
    this.container.className = `file-container ${this.viewMode}-view`;

    if (this.items.length === 0) {
      this.container.innerHTML = `
        <div class="empty-state">
          <div class="empty-icon">📂</div>
          <h3>This folder is empty</h3>
          <p>Tap <b>+</b> to import photos, documents, or create a folder.</p>
        </div>
      `;
      return;
    }

    this.items.forEach((item) => {
      const card = document.createElement('div');
      card.className = `file-card ${this.selectedIds.has(item.id) ? 'selected' : ''}`;
      card.dataset.id = item.id;

      // Icon determination
      const icon = this.getItemIcon(item);
      const formattedSize = item.isFolder ? 'Folder' : this.formatFileSize(item.size || (item.data ? item.data.size : 0));
      const formattedDate = this.formatDate(item.modifiedAt);

      card.innerHTML = `
        <div class="file-checkbox ${this.selectedIds.has(item.id) ? 'checked' : ''}">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
            <polyline points="20 6 9 17 4 12"></polyline>
          </svg>
        </div>
        <div class="file-thumbnail">
          ${icon}
        </div>
        <div class="file-details">
          <div class="file-name" title="${item.name}">${item.name}</div>
          <div class="file-meta">
            <span>${formattedSize}</span>
            <span class="meta-dot">•</span>
            <span>${formattedDate}</span>
          </div>
        </div>
        <button class="file-menu-btn" title="Actions" aria-label="Actions">
          <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
            <circle cx="12" cy="5" r="2"></circle>
            <circle cx="12" cy="12" r="2"></circle>
            <circle cx="12" cy="19" r="2"></circle>
          </svg>
        </button>
      `;

      // Tap handling
      card.onclick = (e) => {
        // If clicked on menu button
        if (e.target.closest('.file-menu-btn')) {
          e.stopPropagation();
          window.app.showFileActionSheet(item);
          return;
        }

        // If clicked on checkbox or in selection mode
        if (e.target.closest('.file-checkbox') || this.isSelectionMode) {
          e.stopPropagation();
          this.toggleSelection(item.id);
          return;
        }

        // Normal tap
        if (item.isFolder) {
          this.navigateToFolder(item.id, item.name);
        } else {
          // Open quick preview or action
          window.app.previewFile(item);
        }
      };

      // Long press handling on mobile
      let pressTimer;
      card.addEventListener('touchstart', () => {
        pressTimer = setTimeout(() => {
          this.isSelectionMode = true;
          this.toggleSelection(item.id);
          navigator.vibrate && navigator.vibrate(30);
        }, 500);
      }, { passive: true });

      card.addEventListener('touchend', () => clearTimeout(pressTimer));
      card.addEventListener('touchmove', () => clearTimeout(pressTimer));

      this.container.appendChild(card);
    });

    if (window.viewsPreviews) {
      window.viewsPreviews.updateStatusBar();
    }
  }

  getItemIcon(item) {
    if (item.isFolder) {
      return `<div class="icon-folder">📁</div>`;
    }

    const cat = item.category || 'other';
    const ext = window.fileeConverter.getExtension(item.name);

    if (cat === 'image') {
      return `<div class="icon-badge badge-image">🖼️ <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
    } else if (cat === 'pdf') {
      return `<div class="icon-badge badge-pdf">📄 <span class="badge-tag">PDF</span></div>`;
    } else if (cat === 'document') {
      return `<div class="icon-badge badge-doc">📝 <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
    } else if (cat === 'audio') {
      return `<div class="icon-badge badge-audio">🎵 <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
    } else if (cat === 'video') {
      return `<div class="icon-badge badge-video">🎬 <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
    } else if (cat === 'archive') {
      return `<div class="icon-badge badge-archive">📦 <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
    } else if (cat === 'data') {
      return `<div class="icon-badge badge-data">📊 <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
    }
    return `<div class="icon-badge badge-other">📄 <span class="badge-tag">${ext.toUpperCase()}</span></div>`;
  }

  toggleSelection(id) {
    if (this.selectedIds.has(id)) {
      this.selectedIds.delete(id);
    } else {
      this.selectedIds.add(id);
    }

    if (this.selectedIds.size === 0) {
      this.isSelectionMode = false;
    } else {
      this.isSelectionMode = true;
    }

    this.updateSelectionBar();
    this.render();
  }

  selectAll() {
    this.items.forEach((item) => this.selectedIds.add(item.id));
    this.isSelectionMode = true;
    this.updateSelectionBar();
    this.render();
  }

  clearSelection() {
    this.selectedIds.clear();
    this.isSelectionMode = false;
    this.updateSelectionBar();
    this.render();
  }

  updateSelectionBar() {
    const bar = document.getElementById('selection-action-bar');
    const countEl = document.getElementById('selection-count');
    if (!bar) return;

    if (this.selectedIds.size > 0) {
      bar.classList.add('visible');
      if (countEl) countEl.textContent = `${this.selectedIds.size} selected`;
    } else {
      bar.classList.remove('visible');
    }
  }

  // --- File System Operations ---
  async createFolder(name) {
    const cleanName = name.trim();
    if (!cleanName) return;

    await window.fileeDB.saveItem({
      name: cleanName,
      isFolder: true,
      parentId: this.currentFolderId,
      modifiedAt: Date.now()
    });

    await this.loadCurrentFolder();
    window.app.showToast(`Folder "${cleanName}" created`, 'success');
  }

  async importFiles(fileList) {
    if (!fileList || fileList.length === 0) return;
    let count = 0;

    for (const rawFile of fileList) {
      const format = window.fileeConverter.getFormat(rawFile.name, rawFile.type);
      await window.fileeDB.saveItem({
        name: rawFile.name,
        isFolder: false,
        parentId: this.currentFolderId,
        size: rawFile.size,
        type: rawFile.type || format.mime,
        category: format.category,
        data: rawFile,
        modifiedAt: rawFile.lastModified || Date.now()
      });
      count++;
    }

    await this.loadCurrentFolder();
    window.app.showToast(`Imported ${count} file${count > 1 ? 's' : ''}`, 'success');
  }

  async deleteItem(id) {
    await window.fileeDB.deleteItem(id);
    await this.loadCurrentFolder();
    window.app.showToast('Item deleted', 'info');
  }

  async renameItem(id, newName) {
    const item = await window.fileeDB.getItem(id);
    if (!item) return;
    item.name = newName.trim();
    item.modifiedAt = Date.now();
    await window.fileeDB.saveItem(item);
    await this.loadCurrentFolder();
    window.app.showToast(`Renamed to "${newName}"`, 'success');
  }

  async saveConvertedFile(blob, originalName, targetExt) {
    // Generate new name: "photo_converted.webp" or "photo.webp"
    const baseName = originalName.substring(0, originalName.lastIndexOf('.')) || originalName;
    const newName = `${baseName}.${targetExt}`;
    const format = window.fileeConverter.getFormat(newName, blob.type);

    // Save in 'Converted' folder if it exists, otherwise current folder
    let targetFolderId = this.currentFolderId;
    const folders = await window.fileeDB.getItemsInFolder('root');
    const convertedFolder = folders.find((f) => f.isFolder && f.name.toLowerCase() === 'converted');
    if (convertedFolder) {
      targetFolderId = convertedFolder.id;
    }

    const saved = await window.fileeDB.saveItem({
      name: newName,
      isFolder: false,
      parentId: targetFolderId,
      size: blob.size,
      type: blob.type || format.mime,
      category: format.category,
      data: blob,
      modifiedAt: Date.now()
    });

    await this.loadCurrentFolder();
    return saved;
  }

  formatFileSize(bytes) {
    if (!bytes || bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(1)) + ' ' + sizes[i];
  }

  formatDate(timestamp) {
    if (!timestamp) return '';
    const date = new Date(timestamp);
    const now = new Date();
    const isToday = date.toDateString() === now.toDateString();
    if (isToday) {
      return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }
    return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
  }
}

window.fileManager = new FileManager();
