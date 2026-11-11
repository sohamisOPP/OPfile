/**
 * OPfile - Persistent Storage Layer (IndexedDB)
 * 100% Offline client-side storage for files, folders, and conversion history.
 */

class FileeDB {
  constructor() {
    this.dbName = 'OPfileDB';
    this.version = 1;
    this.db = null;
  }

  async init() {
    if (this.db) return this.db;
    return new Promise((resolve, reject) => {
      const request = indexedDB.open(this.dbName, this.version);

      request.onupgradeneeded = (e) => {
        const db = e.target.result;

        // Files & Folders Store
        if (!db.objectStoreNames.contains('items')) {
          const itemStore = db.createObjectStore('items', { keyPath: 'id' });
          itemStore.createIndex('parentId', 'parentId', { unique: false });
          itemStore.createIndex('category', 'category', { unique: false });
          itemStore.createIndex('isFolder', 'isFolder', { unique: false });
          itemStore.createIndex('modifiedAt', 'modifiedAt', { unique: false });
        }

        // Conversion History Store
        if (!db.objectStoreNames.contains('history')) {
          const historyStore = db.createObjectStore('history', { keyPath: 'id' });
          historyStore.createIndex('timestamp', 'timestamp', { unique: false });
        }

        // Key-Value Settings Store
        if (!db.objectStoreNames.contains('settings')) {
          db.createObjectStore('settings', { keyPath: 'key' });
        }
      };

      request.onsuccess = async (e) => {
        this.db = e.target.result;
        await this.ensureSeedData();
        resolve(this.db);
      };

      request.onerror = (e) => {
        console.error('IndexedDB open error:', e);
        reject(e);
      };
    });
  }

  async ensureSeedData() {
    // If database is completely empty, add default sample folders & files
    const count = await this.getItemCount();
    if (count === 0) {
      console.log('Seeding initial folders and sample files...');
      const now = Date.now();

      // Default Folders
      await this.saveItem({
        id: 'folder_documents',
        name: 'Documents',
        isFolder: true,
        parentId: 'root',
        modifiedAt: now
      });

      await this.saveItem({
        id: 'folder_images',
        name: 'Images',
        isFolder: true,
        parentId: 'root',
        modifiedAt: now
      });

      await this.saveItem({
        id: 'folder_converted',
        name: 'Converted',
        isFolder: true,
        parentId: 'root',
        modifiedAt: now
      });

      // Sample Welcome Document
      const sampleText = `# Welcome to OPfile 📱\n\nOPfile is your private, 100% offline file manager and multi-format converter.\n\n### Highlights:\n- 🚀 Completely offline: No files ever leave your phone.\n- 🍩 The Donut Converter: Tap any file and convert it in seconds.\n- 🖼️ Image Converter: PNG, JPG, WebP, GIF, BMP, SVG, and ICO.\n- 📄 PDF Converter: Merge, split, images to PDF, PDF to images.\n- 📦 Archives: Create and extract ZIP archives.\n- 🎵 Media & Audio: Extract audio and convert formats.\n- 🗃️ Data: JSON, CSV, XML.\n\nEnjoy your offline privacy!`;
      const blob = new Blob([sampleText], { type: 'text/markdown' });
      await this.saveItem({
        id: 'welcome_file',
        name: 'Welcome to OPfile.md',
        isFolder: false,
        parentId: 'folder_documents',
        size: blob.size,
        type: 'text/markdown',
        category: 'document',
        data: blob,
        modifiedAt: now
      });
    }
  }

  getItemCount() {
    return new Promise((resolve) => {
      const tx = this.db.transaction('items', 'readonly');
      const store = tx.objectStore('items');
      const countReq = store.count();
      countReq.onsuccess = () => resolve(countReq.result);
      countReq.onerror = () => resolve(0);
    });
  }

  async getItemsInFolder(folderId = 'root') {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('items', 'readonly');
      const store = tx.objectStore('items');
      const index = store.index('parentId');
      const req = index.getAll(folderId);

      req.onsuccess = () => {
        const items = req.result || [];
        // Sort: folders first, then alphabetical
        items.sort((a, b) => {
          if (a.isFolder && !b.isFolder) return -1;
          if (!a.isFolder && b.isFolder) return 1;
          return a.name.localeCompare(b.name);
        });
        resolve(items);
      };
      req.onerror = () => reject(req.error);
    });
  }

  async getItem(id) {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('items', 'readonly');
      const store = tx.objectStore('items');
      const req = store.get(id);
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }

  async saveItem(item) {
    if (!item.id) {
      item.id = 'item_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
    }
    if (!item.modifiedAt) {
      item.modifiedAt = Date.now();
    }
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('items', 'readwrite');
      const store = tx.objectStore('items');
      const req = store.put(item);
      req.onsuccess = () => resolve(item);
      req.onerror = () => reject(req.error);
    });
  }

  async deleteItem(id) {
    // If it's a folder, recursively delete children
    const item = await this.getItem(id);
    if (!item) return;

    if (item.isFolder) {
      const children = await this.getItemsInFolder(id);
      for (const child of children) {
        await this.deleteItem(child.id);
      }
    }

    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('items', 'readwrite');
      const store = tx.objectStore('items');
      const req = store.delete(id);
      req.onsuccess = () => resolve(true);
      req.onerror = () => reject(req.error);
    });
  }

  async getAllFiles() {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('items', 'readonly');
      const store = tx.objectStore('items');
      const req = store.getAll();
      req.onsuccess = () => {
        const all = req.result || [];
        resolve(all.filter((i) => !i.isFolder));
      };
      req.onerror = () => reject(req.error);
    });
  }

  async searchFiles(query) {
    const q = query.toLowerCase().trim();
    if (!q) return [];
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('items', 'readonly');
      const store = tx.objectStore('items');
      const req = store.getAll();
      req.onsuccess = () => {
        const all = req.result || [];
        const matches = all.filter((item) => item.name.toLowerCase().includes(q));
        resolve(matches);
      };
      req.onerror = () => reject(req.error);
    });
  }

  // --- Conversion History ---
  async addHistory(entry) {
    if (!entry.id) {
      entry.id = 'hist_' + Date.now() + '_' + Math.random().toString(36).substr(2, 6);
    }
    entry.timestamp = Date.now();
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('history', 'readwrite');
      const store = tx.objectStore('history');
      const req = store.put(entry);
      req.onsuccess = () => resolve(entry);
      req.onerror = () => reject(req.error);
    });
  }

  async getHistory(limit = 50) {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('history', 'readonly');
      const store = tx.objectStore('history');
      const req = store.getAll();
      req.onsuccess = () => {
        const list = req.result || [];
        list.sort((a, b) => b.timestamp - a.timestamp);
        resolve(list.slice(0, limit));
      };
      req.onerror = () => reject(req.error);
    });
  }

  async clearHistory() {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('history', 'readwrite');
      const store = tx.objectStore('history');
      const req = store.clear();
      req.onsuccess = () => resolve(true);
      req.onerror = () => reject(req.error);
    });
  }

  // --- Settings ---
  async getSetting(key, defaultValue = null) {
    return new Promise((resolve) => {
      const tx = this.db.transaction('settings', 'readonly');
      const store = tx.objectStore('settings');
      const req = store.get(key);
      req.onsuccess = () => {
        resolve(req.result ? req.result.value : defaultValue);
      };
      req.onerror = () => resolve(defaultValue);
    });
  }

  async setSetting(key, value) {
    return new Promise((resolve, reject) => {
      const tx = this.db.transaction('settings', 'readwrite');
      const store = tx.objectStore('settings');
      const req = store.put({ key, value });
      req.onsuccess = () => resolve(value);
      req.onerror = () => reject(req.error);
    });
  }

  // Total Storage Used
  async getStorageStats() {
    const files = await this.getAllFiles();
    let totalBytes = 0;
    const byCategory = {
      image: 0,
      document: 0,
      pdf: 0,
      audio: 0,
      video: 0,
      archive: 0,
      other: 0
    };

    files.forEach((file) => {
      const size = file.size || (file.data ? file.data.size : 0);
      totalBytes += size;
      const cat = byCategory[file.category] !== undefined ? file.category : 'other';
      byCategory[cat] += size;
    });

    return { totalBytes, byCategory, fileCount: files.length };
  }
}

window.fileeDB = new FileeDB();
