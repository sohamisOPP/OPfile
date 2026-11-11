/**
 * Filee Mobile - Complete Offline Conversion Engines
 * 100% Client-Side. No network calls, completely offline and private.
 */

class FileeConverter {
  constructor() {
    this.formats = {
      // Images
      png: { id: 'png', name: 'PNG Image', category: 'image', mime: 'image/png', ext: 'png' },
      jpg: { id: 'jpg', name: 'JPEG Image', category: 'image', mime: 'image/jpeg', ext: 'jpg' },
      jpeg: { id: 'jpeg', name: 'JPEG Image', category: 'image', mime: 'image/jpeg', ext: 'jpg' },
      webp: { id: 'webp', name: 'WebP Image', category: 'image', mime: 'image/webp', ext: 'webp' },
      bmp: { id: 'bmp', name: 'BMP Bitmap', category: 'image', mime: 'image/bmp', ext: 'bmp' },
      gif: { id: 'gif', name: 'GIF Animation', category: 'image', mime: 'image/gif', ext: 'gif' },
      svg: { id: 'svg', name: 'SVG Vector', category: 'image', mime: 'image/svg+xml', ext: 'svg' },
      ico: { id: 'ico', name: 'Windows Icon', category: 'image', mime: 'image/x-icon', ext: 'ico' },

      // Documents & PDF
      pdf: { id: 'pdf', name: 'PDF Document', category: 'pdf', mime: 'application/pdf', ext: 'pdf' },
      txt: { id: 'txt', name: 'Plain Text', category: 'document', mime: 'text/plain', ext: 'txt' },
      md: { id: 'md', name: 'Markdown File', category: 'document', mime: 'text/markdown', ext: 'md' },
      html: { id: 'html', name: 'HTML Webpage', category: 'document', mime: 'text/html', ext: 'html' },

      // Audio
      wav: { id: 'wav', name: 'WAV Audio', category: 'audio', mime: 'audio/wav', ext: 'wav' },
      mp3: { id: 'mp3', name: 'MP3 Audio', category: 'audio', mime: 'audio/mpeg', ext: 'mp3' },
      ogg: { id: 'ogg', name: 'OGG Audio', category: 'audio', mime: 'audio/ogg', ext: 'ogg' },
      webm_audio: { id: 'webm_audio', name: 'WebM Audio', category: 'audio', mime: 'audio/webm', ext: 'webm' },

      // Video
      mp4: { id: 'mp4', name: 'MP4 Video', category: 'video', mime: 'video/mp4', ext: 'mp4' },
      webm_video: { id: 'webm_video', name: 'WebM Video', category: 'video', mime: 'video/webm', ext: 'webm' },

      // Archives
      zip: { id: 'zip', name: 'ZIP Archive', category: 'archive', mime: 'application/zip', ext: 'zip' },
      tar: { id: 'tar', name: 'TAR Archive', category: 'archive', mime: 'application/x-tar', ext: 'tar' },

      // Data
      json: { id: 'json', name: 'JSON Data', category: 'data', mime: 'application/json', ext: 'json' },
      csv: { id: 'csv', name: 'CSV Spreadsheet', category: 'data', mime: 'text/csv', ext: 'csv' },
      xml: { id: 'xml', name: 'XML Data', category: 'data', mime: 'application/xml', ext: 'xml' }
    };

    // Configure PDF.js worker if available
    if (window.pdfjsLib) {
      window.pdfjsLib.GlobalWorkerOptions.workerSrc = '/libs/pdf.worker.min.js';
    }
  }

  // --- Format Detection ---
  getExtension(filename) {
    if (!filename) return '';
    const parts = filename.toLowerCase().split('.');
    return parts.length > 1 ? parts.pop() : '';
  }

  getFormat(filename, mimeType = '') {
    const ext = this.getExtension(filename);
    if (this.formats[ext]) {
      return this.formats[ext];
    }
    // Fallback detection from mime
    for (const key in this.formats) {
      if (this.formats[key].mime === mimeType) {
        return this.formats[key];
      }
    }
    return { id: ext || 'unknown', name: ext ? ext.toUpperCase() : 'Unknown File', category: 'other', mime: mimeType, ext: ext };
  }

  // Get available target formats for a file
  getTargetFormats(sourceFormat) {
    const src = sourceFormat.toLowerCase();
    const targets = [];

    // Image Source
    if (['png', 'jpg', 'jpeg', 'webp', 'bmp', 'gif', 'svg', 'ico'].includes(src)) {
      targets.push(
        { id: 'png', name: 'PNG', category: 'image', desc: 'Lossless web standard' },
        { id: 'jpg', name: 'JPEG', category: 'image', desc: 'Compressed photo' },
        { id: 'webp', name: 'WebP', category: 'image', desc: 'Modern high compression' },
        { id: 'bmp', name: 'BMP', category: 'image', desc: 'Raw bitmap' },
        { id: 'pdf', name: 'PDF', category: 'pdf', desc: 'Convert image to PDF document' },
        { id: 'ico', name: 'ICO', category: 'image', desc: 'Favicon / App Icon' }
      );
    }
    // PDF Source
    else if (src === 'pdf') {
      targets.push(
        { id: 'png', name: 'PNG Images', category: 'image', desc: 'Render pages as images' },
        { id: 'jpg', name: 'JPEG Images', category: 'image', desc: 'Render pages as photos' },
        { id: 'txt', name: 'Extracted Text', category: 'document', desc: 'Extract all text' },
        { id: 'zip', name: 'ZIP Archive', category: 'archive', desc: 'Package pages in ZIP' }
      );
    }
    // Text / Markdown Source
    else if (['txt', 'md'].includes(src)) {
      targets.push(
        { id: 'html', name: 'HTML Webpage', category: 'document', desc: 'Render to styled HTML' },
        { id: 'pdf', name: 'PDF Document', category: 'pdf', desc: 'Generate PDF book/doc' },
        { id: 'txt', name: 'Plain Text', category: 'document', desc: 'Strip formatting' }
      );
    }
    // Audio / Video Source
    else if (['mp3', 'wav', 'ogg', 'm4a', 'aac', 'flac'].includes(src)) {
      targets.push(
        { id: 'wav', name: 'WAV Audio', category: 'audio', desc: 'Lossless 16-bit PCM' },
        { id: 'zip', name: 'ZIP Archive', category: 'archive', desc: 'Compress audio' }
      );
    }
    else if (['mp4', 'webm', 'mov', 'mkv', 'avi'].includes(src)) {
      targets.push(
        { id: 'wav', name: 'WAV Audio', category: 'audio', desc: 'Extract soundtrack' },
        { id: 'jpg', name: 'Snapshot (JPG)', category: 'image', desc: 'Extract video frame' },
        { id: 'png', name: 'Snapshot (PNG)', category: 'image', desc: 'High-res frame capture' }
      );
    }
    // Archive Source
    else if (['zip'].includes(src)) {
      targets.push(
        { id: 'extract', name: 'Extract All', category: 'archive', desc: 'Unpack files to folder' }
      );
    }
    // Data Source
    else if (['json', 'csv', 'xml'].includes(src)) {
      if (src === 'json') {
        targets.push(
          { id: 'csv', name: 'CSV Spreadsheet', category: 'data', desc: 'Tabular export' },
          { id: 'xml', name: 'XML Data', category: 'data', desc: 'Structured XML' }
        );
      } else if (src === 'csv') {
        targets.push(
          { id: 'json', name: 'JSON Objects', category: 'data', desc: 'Structured JSON' },
          { id: 'html', name: 'HTML Table', category: 'document', desc: 'Styled table view' }
        );
      } else if (src === 'xml') {
        targets.push(
          { id: 'json', name: 'JSON Objects', category: 'data', desc: 'Convert tags to JSON' }
        );
      }
    }

    // Always offer ZIP compression for any file
    if (src !== 'zip' && !targets.some(t => t.id === 'zip')) {
      targets.push({ id: 'zip', name: 'ZIP Archive', category: 'archive', desc: 'Compress into ZIP' });
    }

    return targets;
  }

  // --- IMAGE ENGINE ---
  async convertImage(fileBlob, options = {}) {
    const {
      targetFormat = 'png',
      quality = 0.9,
      width = null,
      height = null,
      rotation = 0,
      flipH = false,
      flipV = false,
      grayscale = false
    } = options;

    return new Promise((resolve, reject) => {
      const img = new Image();
      const url = URL.createObjectURL(fileBlob);

      img.onload = () => {
        URL.revokeObjectURL(url);
        try {
          // Calculate Target Dimensions
          let targetW = width || img.naturalWidth || img.width;
          let targetH = height || img.naturalHeight || img.height;

          // If only width is provided, maintain aspect ratio
          if (width && !height) {
            targetH = Math.round((img.naturalHeight / img.naturalWidth) * width);
          } else if (height && !width) {
            targetW = Math.round((img.naturalWidth / img.naturalHeight) * height);
          }

          // Handle 90/270 degree rotation dimension swapping
          const isRotated90 = (rotation % 180 !== 0);
          const canvasW = isRotated90 ? targetH : targetW;
          const canvasH = isRotated90 ? targetW : targetH;

          const canvas = document.createElement('canvas');
          canvas.width = canvasW;
          canvas.height = canvasH;
          const ctx = canvas.getContext('2d');

          // Smooth scaling
          ctx.imageSmoothingEnabled = true;
          ctx.imageSmoothingQuality = 'high';

          // Center for rotation/flipping transforms
          ctx.save();
          ctx.translate(canvasW / 2, canvasH / 2);

          if (rotation) {
            ctx.rotate((rotation * Math.PI) / 180);
          }

          ctx.scale(flipH ? -1 : 1, flipV ? -1 : 1);

          // Draw the image
          ctx.drawImage(img, -targetW / 2, -targetH / 2, targetW, targetH);
          ctx.restore();

          // Apply Grayscale Filter if requested
          if (grayscale) {
            const imgData = ctx.getImageData(0, 0, canvasW, canvasH);
            const data = imgData.data;
            for (let i = 0; i < data.length; i += 4) {
              const gray = 0.299 * data[i] + 0.587 * data[i + 1] + 0.114 * data[i + 2];
              data[i] = gray;
              data[i + 1] = gray;
              data[i + 2] = gray;
            }
            ctx.putImageData(imgData, 0, 0);
          }

          // Output MIME selection
          let mime = 'image/png';
          if (targetFormat === 'jpg' || targetFormat === 'jpeg') {
            mime = 'image/jpeg';
            // Fill white background for transparent images converted to JPEG
            const bgCanvas = document.createElement('canvas');
            bgCanvas.width = canvasW;
            bgCanvas.height = canvasH;
            const bgCtx = bgCanvas.getContext('2d');
            bgCtx.fillStyle = '#FFFFFF';
            bgCtx.fillRect(0, 0, canvasW, canvasH);
            bgCtx.drawImage(canvas, 0, 0);
            bgCanvas.toBlob((blob) => {
              resolve({
                blob,
                width: canvasW,
                height: canvasH,
                format: 'jpg',
                ext: 'jpg',
                mime
              });
            }, mime, quality);
            return;
          } else if (targetFormat === 'webp') {
            mime = 'image/webp';
          } else if (targetFormat === 'bmp') {
            mime = 'image/bmp';
          } else if (targetFormat === 'ico') {
            mime = 'image/x-icon';
          }

          canvas.toBlob((blob) => {
            if (!blob) {
              reject(new Error('Canvas blob generation failed'));
              return;
            }
            resolve({
              blob,
              width: canvasW,
              height: canvasH,
              format: targetFormat,
              ext: targetFormat === 'jpeg' ? 'jpg' : targetFormat,
              mime
            });
          }, mime, quality);
        } catch (err) {
          reject(err);
        }
      };

      img.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error('Failed to load image for conversion'));
      };

      img.src = url;
    });
  }

  // --- PDF & DOCUMENT ENGINES ---
  // 1. Images to PDF
  async imagesToPdf(images, options = {}) {
    if (!window.PDFLib) {
      throw new Error('PDF-Lib library not loaded');
    }
    const { PDFDocument, PageSizes } = window.PDFLib;
    const pdfDoc = await PDFDocument.create();

    const fitToPage = options.fitToPage !== false;

    for (const imgItem of images) {
      const buffer = await imgItem.blob.arrayBuffer();
      let pdfImage;
      const isPng = imgItem.blob.type.includes('png');

      try {
        if (isPng) {
          pdfImage = await pdfDoc.embedPng(buffer);
        } else {
          pdfImage = await pdfDoc.embedJpg(buffer);
        }
      } catch (e) {
        // Fallback: convert via canvas to JPEG
        const converted = await this.convertImage(imgItem.blob, { targetFormat: 'jpg', quality: 0.95 });
        const jpgBuf = await converted.blob.arrayBuffer();
        pdfImage = await pdfDoc.embedJpg(jpgBuf);
      }

      const imgDims = pdfImage.scale(1);
      let page;

      if (fitToPage) {
        // Use A4 (595.28 x 841.89 points)
        page = pdfDoc.addPage(PageSizes.A4);
        const margin = 20;
        const availableW = page.getWidth() - margin * 2;
        const availableH = page.getHeight() - margin * 2;
        const scale = Math.min(availableW / imgDims.width, availableH / imgDims.height);
        const drawW = imgDims.width * scale;
        const drawH = imgDims.height * scale;
        const posX = margin + (availableW - drawW) / 2;
        const posY = margin + (availableH - drawH) / 2;

        page.drawImage(pdfImage, {
          x: posX,
          y: posY,
          width: drawW,
          height: drawH
        });
      } else {
        // Page matches image dimensions exactly
        page = pdfDoc.addPage([imgDims.width, imgDims.height]);
        page.drawImage(pdfImage, {
          x: 0,
          y: 0,
          width: imgDims.width,
          height: imgDims.height
        });
      }
    }

    const pdfBytes = await pdfDoc.save();
    return new Blob([pdfBytes], { type: 'application/pdf' });
  }

  // 2. PDF to Images
  async pdfToImages(pdfBlob, options = {}) {
    if (!window.pdfjsLib) {
      throw new Error('PDF.js library not loaded');
    }
    const { format = 'png', scale = 1.5, maxPages = 50 } = options;
    const arrayBuffer = await pdfBlob.arrayBuffer();
    const pdf = await window.pdfjsLib.getDocument({ data: arrayBuffer }).promise;
    const numPages = Math.min(pdf.numPages, maxPages);
    const pages = [];

    for (let pageNum = 1; pageNum <= numPages; pageNum++) {
      const page = await pdf.getPage(pageNum);
      const viewport = page.getViewport({ scale });

      const canvas = document.createElement('canvas');
      canvas.width = viewport.width;
      canvas.height = viewport.height;
      const ctx = canvas.getContext('2d');

      await page.render({
        canvasContext: ctx,
        viewport
      }).promise;

      const blob = await new Promise((res) => {
        const mime = format === 'jpg' ? 'image/jpeg' : 'image/png';
        canvas.toBlob(res, mime, 0.92);
      });

      pages.push({
        pageNumber: pageNum,
        blob,
        filename: `page_${pageNum}.${format}`
      });
    }

    return pages;
  }

  // 3. Merge Multiple PDFs
  async mergePdfs(pdfBlobs) {
    if (!window.PDFLib) {
      throw new Error('PDF-Lib library not loaded');
    }
    const { PDFDocument } = window.PDFLib;
    const mergedDoc = await PDFDocument.create();

    for (const pdfBlob of pdfBlobs) {
      const buffer = await pdfBlob.arrayBuffer();
      const donorDoc = await PDFDocument.load(buffer);
      const copiedPages = await mergedDoc.copyPages(donorDoc, donorDoc.getPageIndices());
      copiedPages.forEach((page) => mergedDoc.addPage(page));
    }

    const mergedBytes = await mergedDoc.save();
    return new Blob([mergedBytes], { type: 'application/pdf' });
  }

  // 4. Split PDF (Extract Pages)
  async splitPdf(pdfBlob, pageNumbers) {
    if (!window.PDFLib) {
      throw new Error('PDF-Lib library not loaded');
    }
    const { PDFDocument } = window.PDFLib;
    const buffer = await pdfBlob.arrayBuffer();
    const srcDoc = await PDFDocument.load(buffer);
    const subDoc = await PDFDocument.create();

    const indices = pageNumbers.map((p) => p - 1).filter((i) => i >= 0 && i < srcDoc.getPageCount());
    const copiedPages = await subDoc.copyPages(srcDoc, indices);
    copiedPages.forEach((page) => subDoc.addPage(page));

    const subBytes = await subDoc.save();
    return new Blob([subBytes], { type: 'application/pdf' });
  }

  // 5. Extract Text from PDF
  async extractPdfText(pdfBlob) {
    if (!window.pdfjsLib) {
      throw new Error('PDF.js library not loaded');
    }
    const arrayBuffer = await pdfBlob.arrayBuffer();
    const pdf = await window.pdfjsLib.getDocument({ data: arrayBuffer }).promise;
    let fullText = '';

    for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const content = await page.getTextContent();
      const pageStrings = content.items.map((item) => item.str).join(' ');
      fullText += `--- Page ${i} ---\n` + pageStrings + '\n\n';
    }

    return new Blob([fullText], { type: 'text/plain' });
  }

  // --- ARCHIVE ENGINES ---
  // 1. Create ZIP
  async createZip(files, options = {}) {
    if (!window.JSZip) {
      throw new Error('JSZip library not loaded');
    }
    const zip = new window.JSZip();
    const compressionLevel = options.level || 6;

    for (const file of files) {
      const data = file.data instanceof Blob ? await file.data.arrayBuffer() : file.data;
      zip.file(file.name, data);
    }

    const zipBlob = await zip.generateAsync({
      type: 'blob',
      compression: 'DEFLATE',
      compressionOptions: { level: compressionLevel }
    });

    return zipBlob;
  }

  // 2. Extract ZIP
  async extractZip(zipBlob) {
    if (!window.JSZip) {
      throw new Error('JSZip library not loaded');
    }
    const zip = await window.JSZip.loadAsync(zipBlob);
    const extractedFiles = [];

    for (const relativePath in zip.files) {
      const entry = zip.files[relativePath];
      if (!entry.dir) {
        const blob = await entry.async('blob');
        extractedFiles.push({
          name: relativePath.split('/').pop(),
          path: relativePath,
          size: blob.size,
          blob
        });
      }
    }

    return extractedFiles;
  }

  // --- AUDIO & MEDIA ENGINES ---
  // Convert Audio/Video to 16-bit PCM WAV
  async convertToWav(mediaBlob) {
    const audioContext = new (window.AudioContext || window.webkitAudioContext)();
    const arrayBuffer = await mediaBlob.arrayBuffer();
    const audioBuffer = await audioContext.decodeAudioData(arrayBuffer);

    // Encode to WAV format
    const numOfChan = audioBuffer.numberOfChannels;
    const length = audioBuffer.length * numOfChan * 2 + 44;
    const outBuffer = new ArrayBuffer(length);
    const view = new DataView(outBuffer);
    const channels = [];
    let sample = 0;
    let offset = 0;
    let pos = 0;

    // RIFF chunk descriptor
    writeString(view, 0, 'RIFF');
    view.setUint32(4, 36 + audioBuffer.length * numOfChan * 2, true);
    writeString(view, 8, 'WAVE');
    // FMT sub-chunk
    writeString(view, 12, 'fmt ');
    view.setUint32(16, 16, true); // Subchunk1Size (16 for PCM)
    view.setUint16(20, 1, true); // AudioFormat (1 for PCM)
    view.setUint16(22, numOfChan, true);
    view.setUint32(24, audioBuffer.sampleRate, true);
    view.setUint32(28, audioBuffer.sampleRate * 2 * numOfChan, true); // ByteRate
    view.setUint16(32, numOfChan * 2, true); // BlockAlign
    view.setUint16(34, 16, true); // BitsPerSample (16 bits)
    // Data sub-chunk
    writeString(view, 36, 'data');
    view.setUint32(40, audioBuffer.length * numOfChan * 2, true);

    // Write interleaved audio data
    for (let i = 0; i < audioBuffer.numberOfChannels; i++) {
      channels.push(audioBuffer.getChannelData(i));
    }

    offset = 44;
    while (pos < audioBuffer.length) {
      for (let i = 0; i < numOfChan; i++) {
        sample = Math.max(-1, Math.min(1, channels[i][pos]));
        sample = (0.5 + sample < 0 ? sample * 32768 : sample * 32767) | 0;
        view.setInt16(offset, sample, true);
        offset += 2;
      }
      pos++;
    }

    function writeString(v, off, str) {
      for (let i = 0; i < str.length; i++) {
        v.setUint8(off + i, str.charCodeAt(i));
      }
    }

    return new Blob([outBuffer], { type: 'audio/wav' });
  }

  // Extract Still Frame Snapshot from Video
  async extractVideoFrame(videoBlob, timestamp = 1.0, format = 'jpg') {
    return new Promise((resolve, reject) => {
      const video = document.createElement('video');
      video.preload = 'metadata';
      video.muted = true;
      video.playsInline = true;
      const url = URL.createObjectURL(videoBlob);

      video.onloadedmetadata = () => {
        video.currentTime = Math.min(timestamp, video.duration || 1);
      };

      video.onseeked = () => {
        try {
          const canvas = document.createElement('canvas');
          canvas.width = video.videoWidth || 640;
          canvas.height = video.videoHeight || 480;
          const ctx = canvas.getContext('2d');
          ctx.drawImage(video, 0, 0, canvas.width, canvas.height);

          URL.revokeObjectURL(url);
          const mime = format === 'jpg' ? 'image/jpeg' : 'image/png';
          canvas.toBlob((blob) => {
            resolve(blob);
          }, mime, 0.95);
        } catch (e) {
          URL.revokeObjectURL(url);
          reject(e);
        }
      };

      video.onerror = () => {
        URL.revokeObjectURL(url);
        reject(new Error('Failed to load video file'));
      };

      video.src = url;
    });
  }

  // --- DATA & TEXT ENGINES ---
  // 1. JSON to CSV
  jsonToCsv(jsonString) {
    try {
      const data = typeof jsonString === 'string' ? JSON.parse(jsonString) : jsonString;
      const array = Array.isArray(data) ? data : [data];
      if (array.length === 0) return new Blob([''], { type: 'text/csv' });

      const headers = Object.keys(array[0]);
      const csvRows = [headers.join(',')];

      for (const row of array) {
        const values = headers.map((header) => {
          const val = row[header] !== undefined ? row[header] : '';
          const escaped = ('' + val).replace(/"/g, '""');
          return `"${escaped}"`;
        });
        csvRows.push(values.join(','));
      }

      return new Blob([csvRows.join('\n')], { type: 'text/csv' });
    } catch (err) {
      throw new Error('Invalid JSON for CSV conversion: ' + err.message);
    }
  }

  // 2. CSV to JSON
  csvToJson(csvString) {
    const lines = csvString.trim().split(/\r\n|\n/);
    if (lines.length < 2) return new Blob(['[]'], { type: 'application/json' });

    const headers = lines[0].split(',').map((h) => h.trim().replace(/^"|"$/g, ''));
    const result = [];

    for (let i = 1; i < lines.length; i++) {
      if (!lines[i].trim()) continue;
      const values = lines[i].split(',').map((v) => v.trim().replace(/^"|"$/g, ''));
      const obj = {};
      headers.forEach((h, idx) => {
        obj[h] = values[idx] !== undefined ? values[idx] : null;
      });
      result.push(obj);
    }

    return new Blob([JSON.stringify(result, null, 2)], { type: 'application/json' });
  }

  // 3. Markdown to HTML
  markdownToHtml(mdText) {
    // Lightweight built-in markdown parser
    let html = mdText
      .replace(/^### (.*$)/gim, '<h3>$1</h3>')
      .replace(/^## (.*$)/gim, '<h2>$1</h2>')
      .replace(/^# (.*$)/gim, '<h1>$1</h1>')
      .replace(/^\> (.*$)/gim, '<blockquote>$1</blockquote>')
      .replace(/\*\*(.*)\*\*/gim, '<b>$1</b>')
      .replace(/\*(.*)\*/gim, '<i>$1</i>')
      .replace(/!\[(.*?)\]\((.*?)\)/gim, '<img alt="$1" src="$2" />')
      .replace(/\[(.*?)\]\((.*?)\)/gim, '<a href="$2">$1</a>')
      .replace(/\n\n/gim, '<p></p>')
      .replace(/\n/gim, '<br />');

    const styledHtml = `<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <title>Document</title>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; padding: 2rem; max-width: 800px; margin: auto; line-height: 1.6; color: #333; }
    h1, h2, h3 { color: #0d1117; }
    blockquote { border-left: 4px solid #00d2b4; margin: 0; padding-left: 1rem; color: #555; }
    code { background: #f0f0f0; padding: 2px 4px; border-radius: 4px; font-family: monospace; }
  </style>
</head>
<body>
${html}
</body>
</html>`;

    return new Blob([styledHtml], { type: 'text/html' });
  }
}

window.fileeConverter = new FileeConverter();
