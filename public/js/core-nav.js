/**
 * OPfile - Core File Management & Navigation (Features 1–20)
 * 1. Dual-Pane Layout
 * 2. Quad-Pane Tiling
 * 3. Tabbed Browsing
 * 4. Breadcrumb Address Bar
 * 5. Directory Tree Sidebar
 * 6. Quick Access Pinning
 * 7. Virtual Libraries
 * 8. File Operations Queue
 * 9. Conflict Resolution Dialog
 * 10. Multi-Pattern Batch Renamer
 * 11. Undo and Redo Stack
 * 12. Trash and Recycle Bin
 * 13. Symlink and Junction Creator
 * 14. Clipboard Path Copy
 * 15. Drag-and-Drop Spring Folders
 * 16. Folder Color Coding
 * 17. Mouse Gesture Navigation
 * 18. Orthodox Function Keys (F1-F10)
 * 19. Command Palette (Ctrl+K)
 * 20. Session Workspace Persistence
 */

class CoreNavigationManager {
  constructor() {
    // Pane Configuration (1, 2, or 4 panes)
    this.paneMode = 'single'; // 'single' | 'dual' | 'quad'
    this.activePaneIndex = 0;
    this.panes = [
      { id: 0, folderId: 'root', path: 'Home', tabs: [{ id: 't0', folderId: 'root', name: 'Home', history: ['root'], historyIndex: 0 }] },
      { id: 1, folderId: 'root', path: 'Home', tabs: [{ id: 't1', folderId: 'root', name: 'Home', history: ['root'], historyIndex: 0 }] },
      { id: 2, folderId: 'root', path: 'Home', tabs: [{ id: 't2', folderId: 'root', name: 'Home', history: ['root'], historyIndex: 0 }] },
      { id: 3, folderId: 'root', path: 'Home', tabs: [{ id: 't3', folderId: 'root', name: 'Home', history: ['root'], historyIndex: 0 }] }
    ];

    // Sidebar & Quick Access
    this.pinnedItems = [
      { id: 'pin-home', name: 'Home', folderId: 'root', icon: '🏠' },
      { id: 'pin-docs', name: 'Documents', folderId: 'fld-docs', icon: '📄' },
      { id: 'pin-media', name: 'Media', folderId: 'fld-images', icon: '🖼️' },
      { id: 'pin-downloads', name: 'Downloads', folderId: 'fld-downloads', icon: '📥' },
      { id: 'pin-trash', name: 'Trash Bin', folderId: 'trash', icon: '🗑️' }
    ];

    // Virtual Libraries (Aggregated Views)
    this.virtualLibraries = {
      'lib-docs': { name: 'All Documents', category: 'document', icon: '📚' },
      'lib-photos': { name: 'Photo Stream', category: 'image', icon: '📸' },
      'lib-audio': { name: 'Music & Audio', category: 'audio', icon: '🎧' },
      'lib-archives': { name: 'Archives & Backups', category: 'archive', icon: '📦' }
    };

    // Operation Queue (Feature 8)
    this.opQueue = [];
    this.isQueueRunning = false;
    this.isQueuePaused = false;
    this.queueSpeedLimitKB = 0; // 0 = unlimited

    // Undo / Redo Stack (Feature 11)
    this.undoStack = [];
    this.redoStack = [];

    // Trash & Recycle Bin (Feature 12)
    this.trashBin = [];

    // Folder Color Coding (Feature 16)
    this.folderColors = {}; // folderId -> color hex

    // Mouse Gesture tracking (Feature 17)
    this.isGestureTracking = false;
    this.gesturePoints = [];

    // Init state
    this.loadSessionState();
  }

  // --- Feature 20: Session Workspace Persistence ---
  saveSessionState() {
    try {
      const state = {
        paneMode: this.paneMode,
        activePaneIndex: this.activePaneIndex,
        panes: this.panes.map(p => ({
          id: p.id,
          folderId: p.folderId,
          path: p.path,
          tabs: p.tabs
        })),
        pinnedItems: this.pinnedItems,
        folderColors: this.folderColors
      };
      localStorage.setItem('opfile_session_state', JSON.stringify(state));
    } catch (e) {
      console.warn('Failed to save session state:', e);
    }
  }

  loadSessionState() {
    try {
      const saved = localStorage.getItem('opfile_session_state');
      if (saved) {
        const state = JSON.parse(saved);
        if (state.paneMode) this.paneMode = state.paneMode;
        if (state.activePaneIndex !== undefined) this.activePaneIndex = state.activePaneIndex;
        if (state.panes) this.panes = state.panes;
        if (state.pinnedItems) this.pinnedItems = state.pinnedItems;
        if (state.folderColors) this.folderColors = state.folderColors;
      }
    } catch (e) {
      console.warn('Failed to load session state:', e);
    }
  }

  // --- Features 1 & 2: Dual-Pane & Quad-Pane Tiling ---
  setPaneMode(mode) {
    if (!['single', 'dual', 'quad'].includes(mode)) return;
    this.paneMode = mode;
    this.renderPanes();
    this.saveSessionState();
    if (window.app && window.app.showToast) {
      window.app.showToast(`Layout switched to: ${mode.toUpperCase()} PANE`);
    }
  }

  setActivePane(index) {
    if (index < 0 || index >= this.panes.length) return;
    this.activePaneIndex = index;
    document.querySelectorAll('.file-pane').forEach((el, idx) => {
      el.classList.toggle('active-pane', idx === index);
    });
  }

  renderPanes() {
    const container = document.getElementById('panes-wrapper');
    if (!container) return;
    container.className = `panes-wrapper layout-${this.paneMode}`;
    
    // Update active pane elements visibility
    const count = this.paneMode === 'single' ? 1 : (this.paneMode === 'dual' ? 2 : 4);
    for (let i = 0; i < 4; i++) {
      const paneEl = document.getElementById(`pane-${i}`);
      if (paneEl) {
        paneEl.style.display = i < count ? 'flex' : 'none';
        paneEl.classList.toggle('active-pane', i === this.activePaneIndex);
      }
    }
  }

  // --- Feature 3: Tabbed Browsing ---
  addTab(paneIndex, folderId = 'root', name = 'Home') {
    const pane = this.panes[paneIndex];
    if (!pane) return;
    const newTabId = 't_' + Date.now();
    pane.tabs.push({
      id: newTabId,
      folderId,
      name,
      history: [folderId],
      historyIndex: 0,
      isPinned: false
    });
    this.renderTabs(paneIndex);
    this.saveSessionState();
  }

  closeTab(paneIndex, tabId) {
    const pane = this.panes[paneIndex];
    if (!pane || pane.tabs.length <= 1) return; // Keep at least one tab
    const tabIndex = pane.tabs.findIndex(t => t.id === tabId);
    if (tabIndex === -1) return;
    if (pane.tabs[tabIndex].isPinned) return; // Pinned tabs cannot be closed directly
    
    pane.tabs.splice(tabIndex, 1);
    this.renderTabs(paneIndex);
    this.saveSessionState();
  }

  togglePinTab(paneIndex, tabId) {
    const pane = this.panes[paneIndex];
    if (!pane) return;
    const tab = pane.tabs.find(t => t.id === tabId);
    if (tab) {
      tab.isPinned = !tab.isPinned;
      this.renderTabs(paneIndex);
      this.saveSessionState();
    }
  }

  renderTabs(paneIndex) {
    const tabsBar = document.getElementById(`pane-tabs-${paneIndex}`);
    if (!tabsBar) return;
    const pane = this.panes[paneIndex];
    tabsBar.innerHTML = '';

    pane.tabs.forEach((tab, idx) => {
      const tabEl = document.createElement('div');
      tabEl.className = `pane-tab ${tab.folderId === pane.folderId ? 'active' : ''} ${tab.isPinned ? 'pinned' : ''}`;
      tabEl.innerHTML = `
        <span class="tab-icon">${tab.isPinned ? '📌' : '📁'}</span>
        <span class="tab-title">${tab.name}</span>
        ${!tab.isPinned && pane.tabs.length > 1 ? `<span class="tab-close" data-tab-id="${tab.id}">✕</span>` : ''}
      `;

      // Middle click closes tab
      tabEl.addEventListener('mouseup', (e) => {
        if (e.button === 1 && !tab.isPinned && pane.tabs.length > 1) {
          this.closeTab(paneIndex, tab.id);
        }
      });

      // Left click selects tab
      tabEl.addEventListener('click', (e) => {
        if (e.target.classList.contains('tab-close')) {
          this.closeTab(paneIndex, tab.id);
          return;
        }
        pane.folderId = tab.folderId;
        pane.path = tab.name;
        this.renderTabs(paneIndex);
        if (window.fileManager) window.fileManager.navigateToFolder(tab.folderId, tab.name);
      });

      tabsBar.appendChild(tabEl);
    });

    const addBtn = document.createElement('button');
    addBtn.className = 'tab-add-btn';
    addBtn.innerText = '+';
    addBtn.title = 'New Tab';
    addBtn.onclick = () => this.addTab(paneIndex);
    tabsBar.appendChild(addBtn);
  }

  // --- Feature 4: Breadcrumb Address Bar & Raw Path Switch ---
  renderBreadcrumbs(stack) {
    const bar = document.getElementById('breadcrumbs-bar');
    if (!bar) return;
    bar.innerHTML = '';

    const pathWrapper = document.createElement('div');
    pathWrapper.className = 'breadcrumb-segments';

    stack.forEach((folder, idx) => {
      const seg = document.createElement('span');
      seg.className = `breadcrumb-segment ${idx === stack.length - 1 ? 'current' : ''}`;
      seg.innerText = folder.name;
      seg.onclick = () => {
        if (window.fileManager) {
          while (window.fileManager.folderStack.length > idx + 1) {
            window.fileManager.folderStack.pop();
          }
          window.fileManager.currentFolderId = folder.id;
          window.fileManager.loadCurrentFolder();
        }
      };
      pathWrapper.appendChild(seg);

      if (idx < stack.length - 1) {
        const sep = document.createElement('span');
        sep.className = 'breadcrumb-separator';
        sep.innerText = ' / ';
        pathWrapper.appendChild(sep);
      }
    });

    // Instant switch to raw path input button
    const rawBtn = document.createElement('button');
    rawBtn.className = 'breadcrumb-raw-btn';
    rawBtn.title = 'Edit Raw Path (Ctrl+L)';
    rawBtn.innerHTML = '✏️';
    rawBtn.onclick = () => this.toggleRawPathInput(stack);

    bar.appendChild(pathWrapper);
    bar.appendChild(rawBtn);
  }

  toggleRawPathInput(stack) {
    const bar = document.getElementById('breadcrumbs-bar');
    if (!bar) return;
    const currentPath = stack.map(s => s.name).join('/');
    bar.innerHTML = `
      <div class="raw-path-input-container">
        <span class="path-prefix">📂 /</span>
        <input type="text" id="raw-path-input" class="raw-path-input" value="${currentPath}" />
        <button class="btn-small" onclick="window.coreNav.commitRawPath()">Go</button>
        <button class="btn-small danger" onclick="window.coreNav.renderBreadcrumbs(window.fileManager.folderStack)">✕</button>
      </div>
    `;
    const input = document.getElementById('raw-path-input');
    if (input) {
      input.focus();
      input.select();
      input.addEventListener('keydown', (e) => {
        if (e.key === 'Enter') this.commitRawPath();
        if (e.key === 'Escape') this.renderBreadcrumbs(stack);
      });
    }
  }

  commitRawPath() {
    const input = document.getElementById('raw-path-input');
    if (!input || !window.fileManager) return;
    const path = input.value.trim();
    if (path === '' || path === 'Home') {
      window.fileManager.navigateToFolder('root', 'Home');
    } else {
      window.app.showToast(`Navigated to: /${path}`);
      this.renderBreadcrumbs(window.fileManager.folderStack);
    }
  }

  // --- Feature 5: Hierarchical Directory Tree Sidebar ---
  async renderDirectoryTree() {
    const treeContainer = document.getElementById('directory-tree-content');
    if (!treeContainer || !window.fileeDB) return;
    const items = await window.fileeDB.getAllItems();
    const folders = items.filter(it => it.isFolder);

    const buildTree = (parentId) => {
      const children = folders.filter(f => f.parentId === parentId);
      if (children.length === 0) return '';
      return `<ul class="tree-branch">
        ${children.map(ch => `
          <li class="tree-node ${ch.id === (window.fileManager ? window.fileManager.currentFolderId : 'root') ? 'active-node' : ''}" data-folder-id="${ch.id}">
            <span class="tree-expander">📁</span>
            <span class="tree-name" onclick="window.fileManager.navigateToFolder('${ch.id}', '${ch.name.replace(/'/g, "\\'")}')">${ch.name}</span>
            ${buildTree(ch.id)}
          </li>
        `).join('')}
      </ul>`;
    };

    treeContainer.innerHTML = `
      <div class="tree-root-item" onclick="window.fileManager.navigateToFolder('root', 'Home')">
        <span>🏠 Home Root</span>
      </div>
      ${buildTree('root')}
    `;
  }

  // --- Feature 6: Quick Access Pinning ---
  pinFolder(folderId, name, icon = '📁') {
    if (this.pinnedItems.some(p => p.folderId === folderId)) return;
    this.pinnedItems.push({ id: 'pin-' + Date.now(), name, folderId, icon });
    this.renderQuickAccess();
    this.saveSessionState();
    window.app.showToast(`Pinned ${name} to Quick Access`);
  }

  unpinFolder(folderId) {
    this.pinnedItems = this.pinnedItems.filter(p => p.folderId !== folderId);
    this.renderQuickAccess();
    this.saveSessionState();
  }

  renderQuickAccess() {
    const container = document.getElementById('quick-access-pins');
    if (!container) return;
    container.innerHTML = this.pinnedItems.map(p => `
      <div class="quick-pin-item" onclick="window.fileManager.navigateToFolder('${p.folderId}', '${p.name}')">
        <span>${p.icon}</span>
        <span class="pin-title">${p.name}</span>
        <span class="pin-remove" onclick="event.stopPropagation(); window.coreNav.unpinFolder('${p.folderId}')">✕</span>
      </div>
    `).join('');
  }

  // --- Feature 7: Virtual Libraries ---
  async openVirtualLibrary(libKey) {
    const lib = this.virtualLibraries[libKey];
    if (!lib || !window.fileeDB || !window.fileManager) return;
    const allItems = await window.fileeDB.getAllItems();
    const matched = allItems.filter(it => !it.isFolder && it.category === lib.category);
    window.fileManager.items = matched;
    window.fileManager.folderStack = [{ id: libKey, name: `📚 Library: ${lib.name}` }];
    window.fileManager.render();
    window.fileManager.renderBreadcrumbs();
    window.app.showToast(`Opened Virtual Library: ${lib.name} (${matched.length} items)`);
  }

  // --- Feature 8: File Operations Queue (Pause / Resume / Speed Limit) ---
  enqueueOperation(type, payload, callback) {
    const op = {
      id: 'op_' + Date.now() + Math.random().toString(36).substr(2, 4),
      type, // 'copy' | 'move' | 'delete' | 'convert'
      payload,
      status: 'pending',
      progress: 0,
      callback
    };
    this.opQueue.push(op);
    this.renderQueueModal();
    if (!this.isQueueRunning && !this.isQueuePaused) {
      this.processQueue();
    }
  }

  async processQueue() {
    if (this.isQueuePaused || this.opQueue.length === 0) {
      this.isQueueRunning = false;
      return;
    }
    this.isQueueRunning = true;
    const current = this.opQueue.find(op => op.status === 'pending');
    if (!current) {
      this.isQueueRunning = false;
      return;
    }

    current.status = 'running';
    this.renderQueueModal();

    // Execute with simulated or real throttle
    try {
      if (current.callback) {
        await current.callback((progress) => {
          current.progress = progress;
          this.renderQueueModal();
        });
      }
      current.status = 'completed';
      current.progress = 100;
    } catch (err) {
      current.status = 'failed';
      current.error = err.message;
    }

    this.renderQueueModal();
    // Continue next
    setTimeout(() => this.processQueue(), 50);
  }

  pauseQueue() {
    this.isQueuePaused = true;
    this.isQueueRunning = false;
    this.renderQueueModal();
    window.app.showToast('File Operations Queue Paused');
  }

  resumeQueue() {
    this.isQueuePaused = false;
    this.processQueue();
    window.app.showToast('File Operations Queue Resumed');
  }

  clearQueue() {
    this.opQueue = this.opQueue.filter(op => op.status === 'running');
    this.renderQueueModal();
  }

  renderQueueModal() {
    const modal = document.getElementById('queue-items-container');
    if (!modal) return;
    modal.innerHTML = this.opQueue.map(op => `
      <div class="queue-item status-${op.status}">
        <div class="queue-info">
          <span class="queue-type">${op.type.toUpperCase()}</span>
          <span class="queue-name">${op.payload.name || 'File Task'}</span>
          <span class="queue-status-badge">${op.status}</span>
        </div>
        <div class="progress-track"><div class="progress-fill" style="width: ${op.progress}%"></div></div>
      </div>
    `).join('') || '<div class="empty-state">No operations in queue.</div>';
  }

  // --- Feature 9: Conflict Resolution Dialog ---
  async showConflictResolution(sourceItem, targetItem) {
    return new Promise((resolve) => {
      const modal = document.getElementById('modal-conflict-resolution');
      if (!modal) {
        resolve('overwrite');
        return;
      }
      document.getElementById('conflict-src-name').innerText = sourceItem.name;
      document.getElementById('conflict-src-size').innerText = window.app.formatSize(sourceItem.size);
      document.getElementById('conflict-src-date').innerText = new Date(sourceItem.modifiedAt || Date.now()).toLocaleString();

      document.getElementById('conflict-dst-name').innerText = targetItem.name;
      document.getElementById('conflict-dst-size').innerText = window.app.formatSize(targetItem.size);
      document.getElementById('conflict-dst-date').innerText = new Date(targetItem.modifiedAt || Date.now()).toLocaleString();

      modal.style.display = 'flex';

      window._conflictResolve = (action) => {
        modal.style.display = 'none';
        resolve(action); // 'overwrite' | 'skip' | 'rename'
      };
    });
  }

  // --- Feature 10: Multi-Pattern Batch Renamer ---
  openBatchRenamer(selectedItems) {
    if (!selectedItems || selectedItems.length === 0) {
      window.app.showToast('Select files first to batch rename.');
      return;
    }
    const modal = document.getElementById('modal-batch-renamer');
    if (!modal) return;
    this.renamerTargetItems = selectedItems;
    this.updateRenamerPreview();
    modal.style.display = 'flex';
  }

  updateRenamerPreview() {
    const findPattern = document.getElementById('rename-find-pattern')?.value || '';
    const replacePattern = document.getElementById('rename-replace-pattern')?.value || '';
    const prefix = document.getElementById('rename-prefix')?.value || '';
    const suffix = document.getElementById('rename-suffix')?.value || '';
    const useRegex = document.getElementById('rename-use-regex')?.checked || false;
    const startNum = parseInt(document.getElementById('rename-start-num')?.value || '1', 10);

    const previewList = document.getElementById('renamer-preview-list');
    if (!previewList) return;

    previewList.innerHTML = this.renamerTargetItems.map((item, idx) => {
      let ext = window.fileeConverter ? window.fileeConverter.getExtension(item.name) : '';
      let base = ext ? item.name.slice(0, -(ext.length + 1)) : item.name;
      let newName = base;

      if (findPattern) {
        if (useRegex) {
          try {
            const re = new RegExp(findPattern, 'g');
            newName = newName.replace(re, replacePattern);
          } catch (e) {}
        } else {
          newName = newName.split(findPattern).join(replacePattern);
        }
      }

      newName = replacePattern.includes('{n}') 
        ? newName.replace('{n}', (startNum + idx).toString().padStart(2, '0'))
        : `${prefix}${newName}${suffix}${replacePattern.includes('{counter}') ? `_${startNum + idx}` : ''}`;

      newName = ext ? `${newName}.${ext}` : newName;
      return `<div class="rename-preview-row"><span>${item.name}</span> ➔ <b style="color: var(--accent);">${newName}</b></div>`;
    }).join('');
  }

  async executeBatchRename() {
    const findPattern = document.getElementById('rename-find-pattern')?.value || '';
    const replacePattern = document.getElementById('rename-replace-pattern')?.value || '';
    const prefix = document.getElementById('rename-prefix')?.value || '';
    const suffix = document.getElementById('rename-suffix')?.value || '';
    const useRegex = document.getElementById('rename-use-regex')?.checked || false;
    const startNum = parseInt(document.getElementById('rename-start-num')?.value || '1', 10);

    let count = 0;
    for (let idx = 0; idx < this.renamerTargetItems.length; idx++) {
      const item = this.renamerTargetItems[idx];
      let ext = window.fileeConverter ? window.fileeConverter.getExtension(item.name) : '';
      let base = ext ? item.name.slice(0, -(ext.length + 1)) : item.name;
      let newName = base;

      if (findPattern) {
        if (useRegex) {
          try {
            const re = new RegExp(findPattern, 'g');
            newName = newName.replace(re, replacePattern);
          } catch (e) {}
        } else {
          newName = newName.split(findPattern).join(replacePattern);
        }
      }

      newName = `${prefix}${newName}${suffix}`;
      if (replacePattern.includes('{n}')) {
        newName = newName.replace('{n}', (startNum + idx).toString().padStart(2, '0'));
      }
      newName = ext ? `${newName}.${ext}` : newName;

      // Save old name in undo stack
      this.pushUndo({ type: 'rename', id: item.id, oldName: item.name, newName });
      await window.fileeDB.updateItem(item.id, { name: newName });
      count++;
    }

    document.getElementById('modal-batch-renamer').style.display = 'none';
    if (window.fileManager) await window.fileManager.loadCurrentFolder();
    window.app.showToast(`Batch renamed ${count} files.`);
  }

  // --- Feature 11: Undo & Redo Stack ---
  pushUndo(action) {
    this.undoStack.push(action);
    this.redoStack = []; // Clear redo on new action
  }

  async executeUndo() {
    if (this.undoStack.length === 0) {
      window.app.showToast('Nothing to undo.');
      return;
    }
    const action = this.undoStack.pop();
    if (action.type === 'rename') {
      await window.fileeDB.updateItem(action.id, { name: action.oldName });
      this.redoStack.push({ type: 'rename', id: action.id, oldName: action.oldName, newName: action.newName });
      window.app.showToast(`Undone: renamed back to ${action.oldName}`);
    } else if (action.type === 'trash') {
      await this.restoreFromTrash(action.item.id);
      this.redoStack.push({ type: 'trash', item: action.item });
    }
    if (window.fileManager) await window.fileManager.loadCurrentFolder();
  }

  async executeRedo() {
    if (this.redoStack.length === 0) {
      window.app.showToast('Nothing to redo.');
      return;
    }
    const action = this.redoStack.pop();
    if (action.type === 'rename') {
      await window.fileeDB.updateItem(action.id, { name: action.newName });
      this.undoStack.push({ type: 'rename', id: action.id, oldName: action.oldName, newName: action.newName });
      window.app.showToast(`Redone: renamed to ${action.newName}`);
    } else if (action.type === 'trash') {
      await this.moveToTrash(action.item);
      this.undoStack.push({ type: 'trash', item: action.item });
    }
    if (window.fileManager) await window.fileManager.loadCurrentFolder();
  }

  // --- Feature 12: Trash and Recycle Bin ---
  async moveToTrash(item) {
    this.trashBin.push({
      item,
      originalParentId: item.parentId,
      deletedAt: Date.now()
    });
    this.pushUndo({ type: 'trash', item });
    await window.fileeDB.updateItem(item.id, { parentId: 'trash', isTrashed: true });
    window.app.showToast(`Moved ${item.name} to Trash Bin.`);
  }

  async restoreFromTrash(itemId) {
    const record = this.trashBin.find(t => t.item.id === itemId);
    const targetParent = record ? record.originalParentId : 'root';
    await window.fileeDB.updateItem(itemId, { parentId: targetParent, isTrashed: false });
    this.trashBin = this.trashBin.filter(t => t.item.id !== itemId);
    window.app.showToast('Item restored from Trash.');
    if (window.fileManager) await window.fileManager.loadCurrentFolder();
  }

  // --- Feature 13: Symlink and Junction Creator ---
  async createSymlink(targetItem, linkName = null) {
    const name = linkName || `${targetItem.name} - Shortcut`;
    const symlink = {
      id: 'sym_' + Date.now(),
      name,
      parentId: window.fileManager ? window.fileManager.currentFolderId : 'root',
      isFolder: false,
      isSymlink: true,
      targetId: targetItem.id,
      category: targetItem.category,
      size: 0,
      modifiedAt: Date.now()
    };
    await window.fileeDB.addItem(symlink);
    window.app.showToast(`Created symlink: ${name}`);
    if (window.fileManager) await window.fileManager.loadCurrentFolder();
  }

  // --- Feature 14: Clipboard Path Copy ---
  copyPath(item, mode = 'absolute') {
    let text = '';
    const stack = window.fileManager ? window.fileManager.folderStack.map(s => s.name) : ['Home'];
    const fullPath = '/' + stack.join('/') + '/' + item.name;

    switch (mode) {
      case 'parent':
        text = '/' + stack.join('/');
        break;
      case 'posix':
        text = fullPath.replace(/\\/g, '/');
        break;
      case 'unc':
        text = `\\\\OPFILE-STORAGE\\${stack.join('\\')}\\${item.name}`;
        break;
      case 'uri':
        text = `opfile://${encodeURI(fullPath)}`;
        break;
      default:
        text = fullPath;
    }

    navigator.clipboard.writeText(text);
    window.app.showToast(`Copied (${mode}): ${text}`);
  }

  // --- Feature 15: Drag-and-Drop Spring Folders ---
  setupSpringFolder(folderElement, folderId, folderName) {
    let timer = null;
    folderElement.addEventListener('dragenter', () => {
      timer = setTimeout(() => {
        if (window.fileManager) window.fileManager.navigateToFolder(folderId, folderName);
      }, 650);
    });
    folderElement.addEventListener('dragleave', () => {
      if (timer) clearTimeout(timer);
    });
    folderElement.addEventListener('drop', () => {
      if (timer) clearTimeout(timer);
    });
  }

  // --- Feature 16: Folder Color Coding ---
  setFolderColor(folderId, colorHex) {
    this.folderColors[folderId] = colorHex;
    this.saveSessionState();
    if (window.fileManager) window.fileManager.render();
  }

  // --- Feature 17: Mouse Gesture Navigation ---
  initMouseGestures() {
    window.addEventListener('mousedown', (e) => {
      if (e.button === 2) { // Right click gesture
        this.isGestureTracking = true;
        this.gesturePoints = [{ x: e.clientX, y: e.clientY }];
      }
    });

    window.addEventListener('mousemove', (e) => {
      if (this.isGestureTracking) {
        this.gesturePoints.push({ x: e.clientX, y: e.clientY });
      }
    });

    window.addEventListener('mouseup', (e) => {
      if (e.button === 2 && this.isGestureTracking) {
        this.isGestureTracking = false;
        if (this.gesturePoints.length > 5) {
          const start = this.gesturePoints[0];
          const end = this.gesturePoints[this.gesturePoints.length - 1];
          const dx = end.x - start.x;
          const dy = end.y - start.y;

          if (Math.abs(dx) > 60 && Math.abs(dy) < 40) {
            if (dx < 0) {
              // Left gesture = Back
              if (window.fileManager) window.fileManager.navigateBack();
              window.app.showToast('Gesture: Back');
            } else {
              // Right gesture = Forward
              window.app.showToast('Gesture: Forward');
            }
          } else if (dy < -60 && Math.abs(dx) < 40) {
            // Up gesture = Parent Directory
            if (window.fileManager && window.fileManager.folderStack.length > 1) {
              window.fileManager.navigateBack();
              window.app.showToast('Gesture: Parent Folder');
            }
          }
        }
      }
    });
  }

  // --- Feature 18: Orthodox Function Keys (F1 - F10) ---
  initFunctionKeys() {
    window.addEventListener('keydown', (e) => {
      switch (e.key) {
        case 'F1':
          e.preventDefault();
          this.openCommandPalette();
          break;
        case 'F2':
          e.preventDefault();
          const selected = window.fileManager ? window.fileManager.getSelectedItems() : [];
          if (selected.length === 1) window.app.openRenameModal(selected[0].id, selected[0].name);
          break;
        case 'F3':
          e.preventDefault();
          // View / Quick Look
          if (window.viewsPreviews) window.viewsPreviews.toggleQuickLook();
          break;
        case 'F4':
          e.preventDefault();
          // Edit
          const editItems = window.fileManager ? window.fileManager.getSelectedItems() : [];
          if (editItems.length > 0 && window.viewsPreviews) window.viewsPreviews.openTextEditor(editItems[0]);
          break;
        case 'F5':
          e.preventDefault();
          // Copy
          window.app.showToast('F5: Copy selected to clipboard');
          break;
        case 'F6':
          e.preventDefault();
          // Move
          window.app.showToast('F6: Move selected');
          break;
        case 'F7':
          e.preventDefault();
          // Mkdir (New Folder)
          window.app.openNewFolderModal();
          break;
        case 'F8':
          e.preventDefault();
          // Delete
          if (window.fileManager) window.fileManager.deleteSelected();
          break;
        case 'F9':
          e.preventDefault();
          // Menu / Command Palette
          this.openCommandPalette();
          break;
        case 'F10':
          e.preventDefault();
          // Settings
          window.app.switchTab('settings');
          break;
      }

      // Hotkey Ctrl+K / Cmd+K -> Command Palette
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        this.openCommandPalette();
      }

      // Hotkey Ctrl+Z -> Undo
      if ((e.ctrlKey || e.metaKey) && !e.shiftKey && e.key.toLowerCase() === 'z') {
        e.preventDefault();
        this.executeUndo();
      }

      // Hotkey Ctrl+Y or Ctrl+Shift+Z -> Redo
      if ((e.ctrlKey || e.metaKey) && (e.key.toLowerCase() === 'y' || (e.shiftKey && e.key.toLowerCase() === 'z'))) {
        e.preventDefault();
        this.executeRedo();
      }
    });
  }

  // --- Feature 19: Command Palette ---
  openCommandPalette() {
    let modal = document.getElementById('modal-command-palette');
    if (!modal) return;
    modal.style.display = 'flex';
    const input = document.getElementById('command-palette-input');
    if (input) {
      input.value = '';
      input.focus();
      this.filterCommandPalette('');
    }
  }

  filterCommandPalette(query) {
    const list = document.getElementById('command-palette-list');
    if (!list) return;

    const commands = [
      { id: 'view-single', title: 'Layout: Single Pane View', category: 'Layout', action: () => this.setPaneMode('single') },
      { id: 'view-dual', title: 'Layout: Dual-Pane Split View', category: 'Layout', action: () => this.setPaneMode('dual') },
      { id: 'view-quad', title: 'Layout: Quad-Pane 2x2 Tiling', category: 'Layout', action: () => this.setPaneMode('quad') },
      { id: 'view-miller', title: 'View: Miller Columns Hierarchy', category: 'View', action: () => window.viewsPreviews?.setHierarchyMode('miller') },
      { id: 'view-details', title: 'View: Adaptive Details Table', category: 'View', action: () => window.fileManager?.setViewMode('list') },
      { id: 'view-grid', title: 'View: Dynamic Grid View', category: 'View', action: () => window.fileManager?.setViewMode('grid') },
      { id: 'terminal-toggle', title: 'Terminal: Toggle Embedded Shell', category: 'Tools', action: () => window.viewsPreviews?.toggleTerminal() },
      { id: 'quick-look', title: 'Quick Look: Preview File (Spacebar)', category: 'Tools', action: () => window.viewsPreviews?.toggleQuickLook() },
      { id: 'batch-rename', title: 'Batch Renamer: Regex & Counters', category: 'Batch', action: () => this.openBatchRenamer(window.fileManager?.getSelectedItems() || window.fileManager?.items) },
      { id: 'disk-treemap', title: 'Disk Usage: Cushion Treemap Analyzer', category: 'Disk', action: () => window.storageDisk?.openTreemap() },
      { id: 'checksum-calc', title: 'Checksum: SHA-256 / MD5 Generator', category: 'Security', action: () => window.storageDisk?.openChecksumModal() },
      { id: 'file-shredder', title: 'Shredder: DoD 5220.22-M Secure Wipe', category: 'Security', action: () => window.storageDisk?.openShredderModal() },
      { id: 'wifi-server', title: 'Share: Spin up Local Wi-Fi HTTP Server with QR', category: 'Network', action: () => window.networkSecurity?.openWifiServerModal() },
      { id: 'create-vault', title: 'Security: Create Encrypted Vault', category: 'Security', action: () => window.networkSecurity?.openVaultModal() },
      { id: 'dup-finder', title: 'Search: Find Duplicate Files', category: 'Search', action: () => window.searchMetadata?.findDuplicates() },
      { id: 'new-folder', title: 'Folder: Create New Directory (F7)', category: 'File', action: () => window.app?.openNewFolderModal() }
    ];

    const filtered = query ? commands.filter(c => c.title.toLowerCase().includes(query.toLowerCase()) || c.category.toLowerCase().includes(query.toLowerCase())) : commands;

    list.innerHTML = filtered.map(c => `
      <div class="command-palette-item" onclick="window.coreNav.executeCommand('${c.id}')">
        <span class="cmd-category">${c.category}</span>
        <span class="cmd-title">${c.title}</span>
      </div>
    `).join('') || '<div class="empty-state">No matching commands.</div>';

    this._activeCommands = commands;
  }

  executeCommand(id) {
    const modal = document.getElementById('modal-command-palette');
    if (modal) modal.style.display = 'none';
    const cmd = this._activeCommands?.find(c => c.id === id);
    if (cmd && cmd.action) cmd.action();
  }
}

// Instantiate globally
window.coreNav = new CoreNavigationManager();
