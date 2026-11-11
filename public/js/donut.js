/**
 * Filee Mobile - The Signature "Donut Menu" Radial Controller
 * Faithful recreation of Filee's iconic circular format converter ring.
 */

class FileeDonut {
  constructor(canvasElement) {
    this.canvas = canvasElement;
    this.ctx = canvasElement.getContext('2d');
    this.items = [];
    this.activeIndex = -1;
    this.onSelectCallback = null;
    this.onCancelCallback = null;
    this.scale = window.devicePixelRatio || 1;

    this.initEvents();
  }

  setItems(items, onSelect, onCancel) {
    this.items = items || [];
    this.onSelectCallback = onSelect;
    this.onCancelCallback = onCancel;
    this.activeIndex = -1;
    this.render();
  }

  render() {
    const width = this.canvas.clientWidth || 300;
    const height = this.canvas.clientHeight || 300;

    // Retina display support
    this.canvas.width = width * this.scale;
    this.canvas.height = height * this.scale;
    this.ctx.resetTransform();
    this.ctx.scale(this.scale, this.scale);

    const ctx = this.ctx;
    const centerX = width / 2;
    const centerY = height / 2;
    const outerRadius = Math.min(centerX, centerY) - 10;
    const innerRadius = outerRadius * 0.42; // Filee's signature hole ratio

    ctx.clearRect(0, 0, width, height);

    if (this.items.length === 0) {
      // Empty donut
      ctx.beginPath();
      ctx.arc(centerX, centerY, outerRadius, 0, Math.PI * 2);
      ctx.arc(centerX, centerY, innerRadius, Math.PI * 2, 0, true);
      ctx.fillStyle = 'rgba(255, 255, 255, 0.05)';
      ctx.fill();
      return;
    }

    const count = this.items.length;
    const anglePerSlice = (Math.PI * 2) / count;
    const gapAngle = count > 1 ? 0.04 : 0; // slice separation

    // Color palette based on format category
    const categoryColors = {
      image: { normal: '#00D2B4', active: '#00F5D4', text: '#0D1117' },
      pdf: { normal: '#FF5E7E', active: '#FF7B96', text: '#FFFFFF' },
      document: { normal: '#3A86FF', active: '#609BFF', text: '#FFFFFF' },
      audio: { normal: '#FFBE0B', active: '#FFD147', text: '#0D1117' },
      video: { normal: '#FB5607', active: '#FF702E', text: '#FFFFFF' },
      archive: { normal: '#8338EC', active: '#9B5DE5', text: '#FFFFFF' },
      data: { normal: '#06D6A0', active: '#2EC4B6', text: '#0D1117' },
      other: { normal: '#6C757D', active: '#ADB5BD', text: '#FFFFFF' }
    };

    // Draw Slices
    for (let i = 0; i < count; i++) {
      const item = this.items[i];
      const startAngle = i * anglePerSlice - Math.PI / 2 + gapAngle / 2;
      const endAngle = (i + 1) * anglePerSlice - Math.PI / 2 - gapAngle / 2;
      const midAngle = (startAngle + endAngle) / 2;
      const isActive = (i === this.activeIndex);

      const colors = categoryColors[item.category] || categoryColors.other;
      const fillColor = isActive ? colors.active : colors.normal;

      ctx.save();
      ctx.beginPath();
      ctx.arc(centerX, centerY, isActive ? outerRadius + 6 : outerRadius, startAngle, endAngle);
      ctx.arc(centerX, centerY, innerRadius, endAngle, startAngle, true);
      ctx.closePath();

      // Shadow on active slice
      if (isActive) {
        ctx.shadowColor = colors.normal;
        ctx.shadowBlur = 18;
      }

      ctx.fillStyle = fillColor;
      ctx.fill();

      // Draw Slice Text & Icon
      const textRadius = (innerRadius + outerRadius) / 2;
      const textX = centerX + Math.cos(midAngle) * textRadius;
      const textY = centerY + Math.sin(midAngle) * textRadius;

      ctx.restore();
      ctx.save();
      ctx.translate(textX, textY);

      ctx.fillStyle = colors.text;
      ctx.font = 'bold 13px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      ctx.fillText(item.name || item.id.toUpperCase(), 0, 0);

      ctx.restore();
    }

    // Draw Center Hole (Cancel / Close Button)
    ctx.beginPath();
    ctx.arc(centerX, centerY, innerRadius - 4, 0, Math.PI * 2);
    ctx.fillStyle = '#161B22';
    ctx.fill();
    ctx.lineWidth = 2;
    ctx.strokeStyle = '#30363D';
    ctx.stroke();

    // Center Text
    ctx.fillStyle = '#E6EDF3';
    ctx.font = 'bold 14px -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif';
    ctx.textAlign = 'center';
    ctx.textBaseline = 'middle';
    if (this.activeIndex >= 0 && this.items[this.activeIndex]) {
      ctx.fillText(this.items[this.activeIndex].name, centerX, centerY - 6);
      ctx.font = '10px sans-serif';
      ctx.fillStyle = '#8B949E';
      ctx.fillText('Tap to convert', centerX, centerY + 10);
    } else {
      ctx.fillText('Select', centerX, centerY - 6);
      ctx.font = '10px sans-serif';
      ctx.fillStyle = '#8B949E';
      ctx.fillText('Format', centerX, centerY + 10);
    }
  }

  getSliceAt(x, y) {
    const width = this.canvas.clientWidth;
    const height = this.canvas.clientHeight;
    const centerX = width / 2;
    const centerY = height / 2;
    const dx = x - centerX;
    const dy = y - centerY;
    const dist = Math.sqrt(dx * dx + dy * dy);
    const outerRadius = Math.min(centerX, centerY) - 10;
    const innerRadius = outerRadius * 0.42;

    // Inside center hole
    if (dist < innerRadius) {
      return { kind: 'center' };
    }

    // Outside donut
    if (dist > outerRadius + 10) {
      return { kind: 'outside' };
    }

    // Calculate angle (0 at top, clockwise)
    let angle = Math.atan2(dy, dx) + Math.PI / 2;
    if (angle < 0) angle += Math.PI * 2;

    const count = this.items.length;
    if (count === 0) return { kind: 'outside' };

    const anglePerSlice = (Math.PI * 2) / count;
    const index = Math.floor(angle / anglePerSlice);

    if (index >= 0 && index < count) {
      return { kind: 'segment', index };
    }

    return { kind: 'outside' };
  }

  initEvents() {
    const handleMove = (e) => {
      const rect = this.canvas.getBoundingClientRect();
      const x = (e.clientX || (e.touches && e.touches[0].clientX)) - rect.left;
      const y = (e.clientY || (e.touches && e.touches[0].clientY)) - rect.top;
      const hit = this.getSliceAt(x, y);

      if (hit.kind === 'segment') {
        if (this.activeIndex !== hit.index) {
          this.activeIndex = hit.index;
          this.render();
        }
      } else {
        if (this.activeIndex !== -1) {
          this.activeIndex = -1;
          this.render();
        }
      }
    };

    const handleSelect = (e) => {
      e.preventDefault();
      const rect = this.canvas.getBoundingClientRect();
      const clientX = e.clientX || (e.changedTouches && e.changedTouches[0].clientX);
      const clientY = e.clientY || (e.changedTouches && e.changedTouches[0].clientY);
      const x = clientX - rect.left;
      const y = clientY - rect.top;
      const hit = this.getSliceAt(x, y);

      if (hit.kind === 'segment' && this.items[hit.index]) {
        if (this.onSelectCallback) {
          this.onSelectCallback(this.items[hit.index]);
        }
      } else if (hit.kind === 'center') {
        if (this.onCancelCallback) {
          this.onCancelCallback();
        }
      }
    };

    this.canvas.addEventListener('mousemove', handleMove);
    this.canvas.addEventListener('touchmove', handleMove, { passive: true });
    this.canvas.addEventListener('click', handleSelect);
    this.canvas.addEventListener('touchend', handleSelect);
  }
}

window.FileeDonut = FileeDonut;
