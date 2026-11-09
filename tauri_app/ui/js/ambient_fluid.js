/**
 * ambient_fluid.js - Дышащая органическая анимация фона (Liquid Glass Fluid)
 * Реагирует на курсор, переключение разделов, выполнение сценариев и статус оборудования.
 */

(function () {
  let canvas = null;
  let ctx = null;
  let animFrameId = null;
  let width = 0;
  let height = 0;

  // Цветовые палитры (Target RGB)
  // Синий (Статика / Все в норме)
  // Желтый (Синхронизация / Загрузка / Выполнение скрипта)
  // Зеленый (Действие успешно завершено)
  // Красный (Ошибка / Разрыв связи / Линия 0V)
  const COLOR_PALETTES = {
    blue: {
      c1: [10, 132, 255, 0.28],   // #0A84FF
      c2: [94, 92, 230, 0.22],    // #5E5CE6
      c3: [100, 181, 246, 0.16],  // #64B5F6
      glow: [10, 132, 255, 0.18]
    },
    amber: {
      c1: [255, 159, 10, 0.35],   // #FF9F0A
      c2: [255, 214, 10, 0.25],   // #FFD60A
      c3: [255, 179, 64, 0.18],
      glow: [255, 159, 10, 0.22]
    },
    green: {
      c1: [48, 209, 88, 0.35],    // #30D158
      c2: [52, 199, 89, 0.22],    // #34C759
      c3: [129, 199, 132, 0.18],
      glow: [48, 209, 88, 0.20]
    },
    red: {
      c1: [255, 69, 58, 0.38],    // #FF453A
      c2: [255, 59, 48, 0.26],    // #FF3B30
      c3: [239, 83, 80, 0.20],
      glow: [255, 69, 58, 0.22]
    }
  };

  let currentMood = "blue";
  let moodReturnTimer = null;

  // Текущие интерполируемые цвета (Current RGBA)
  const curColors = {
    c1: [10, 132, 255, 0.28],
    c2: [94, 92, 230, 0.22],
    c3: [100, 181, 246, 0.16],
    glow: [10, 132, 255, 0.18]
  };

  // Физика курсора и импульсов
  const mouse = {
    x: -1000,
    y: -1000,
    targetX: -1000,
    targetY: -1000,
    radius: 220,
    active: false,
    decayTimer: null
  };

  const ripples = [];
  let time = 0;

  function initAmbientFluid() {
    canvas = document.getElementById("ambient-canvas");
    if (!canvas) {
      canvas = document.createElement("canvas");
      canvas.id = "ambient-canvas";
      canvas.className = "ambient-canvas";
      document.body.prepend(canvas);
    }
    ctx = canvas.getContext("2d", { alpha: true });

    onResize();
    window.addEventListener("resize", onResize, { passive: true });

    // Интеракция с курсором
    window.addEventListener("mousemove", (e) => {
      mouse.targetX = e.clientX;
      mouse.targetY = e.clientY;
      mouse.active = true;
      if (mouse.decayTimer) clearTimeout(mouse.decayTimer);
      mouse.decayTimer = setTimeout(() => {
        mouse.active = false;
      }, 3500);
    }, { passive: true });

    window.addEventListener("mouseleave", () => {
      mouse.active = false;
    });

    // Реакция на клики по вкладкам и кнопкам (плавный импульс волны)
    document.addEventListener("click", (e) => {
      const btn = e.target.closest("button, .hdmi-chip, .seg-btn, .apple-btn");
      if (btn) {
        const rect = btn.getBoundingClientRect();
        triggerRipple(rect.left + rect.width / 2, rect.top + rect.height / 2, 0.85);
      }
    }, { passive: true });

    startAnimation();
  }

  function onResize() {
    if (!canvas) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 2);
    width = window.innerWidth;
    height = window.innerHeight;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);
    canvas.style.width = width + "px";
    canvas.style.height = height + "px";
    if (ctx) ctx.scale(dpr, dpr);
  }

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function updateColorTransition() {
    const target = COLOR_PALETTES[currentMood] || COLOR_PALETTES.blue;
    const speed = 0.055; // Быстрый, но гладкий переход между оттенками

    for (const key of ["c1", "c2", "c3", "glow"]) {
      const cur = curColors[key];
      const tgt = target[key];
      cur[0] = lerp(cur[0], tgt[0], speed);
      cur[1] = lerp(cur[1], tgt[1], speed);
      cur[2] = lerp(cur[2], tgt[2], speed);
      cur[3] = lerp(cur[3], tgt[3], speed);
    }
  }

  function toRgba(arr, alphaMult = 1) {
    const r = Math.round(arr[0]);
    const g = Math.round(arr[1]);
    const b = Math.round(arr[2]);
    const a = (arr[3] * alphaMult).toFixed(3);
    return `rgba(${r}, ${g}, ${b}, ${a})`;
  }

  function triggerRipple(x, y, power = 1.0) {
    ripples.push({
      x: x || width / 2,
      y: y || height - 80,
      radius: 10,
      maxRadius: Math.max(width, height) * 0.75,
      power: power,
      alpha: 0.65
    });
    if (ripples.length > 8) ripples.shift();
  }

  function drawWave(baseY, amp, freq, speed, color1, color2, phaseOffset) {
    ctx.beginPath();
    ctx.moveTo(0, height);

    const step = 14;
    for (let x = 0; x <= width + step; x += step) {
      // Базовая синусоида волны с дыханием
      const breathing = Math.sin(time * 0.45) * 12;
      let y = baseY + breathing + Math.sin(x * freq + time * speed + phaseOffset) * amp;
      y += Math.cos(x * (freq * 0.5) - time * (speed * 0.7)) * (amp * 0.45);

      // Интеракция с курсором (мягкое физическое отталкивание волны)
      if (mouse.active) {
        const dx = x - mouse.x;
        const dy = y - mouse.y;
        const dist = Math.sqrt(dx * dx + dy * dy);
        if (dist < mouse.radius) {
          const factor = (1 - dist / mouse.radius);
          y += Math.sin(factor * Math.PI) * 32 * (dy < 0 ? -1 : 1);
        }
      }

      // Наложение затухающих круговых импульсов (ripples)
      for (let i = 0; i < ripples.length; i++) {
        const r = ripples[i];
        const distR = Math.abs(x - r.x);
        if (distR < r.radius && distR > r.radius - 80) {
          const ripFactor = Math.sin(((r.radius - distR) / 80) * Math.PI) * r.alpha * 18 * r.power;
          y += ripFactor;
        }
      }

      ctx.lineTo(x, y);
    }

    ctx.lineTo(width, height);
    ctx.closePath();

    const grad = ctx.createLinearGradient(0, baseY - 60, width, height);
    grad.addColorStop(0, color1);
    grad.addColorStop(1, color2);
    ctx.fillStyle = grad;
    ctx.fill();
  }

  function render() {
    if (!ctx) return;
    time += 0.016;

    // Плавное сближение реальной позиции мыши с физической точкой
    if (mouse.active) {
      mouse.x = lerp(mouse.x === -1000 ? mouse.targetX : mouse.x, mouse.targetX, 0.12);
      mouse.y = lerp(mouse.y === -1000 ? mouse.targetY : mouse.y, mouse.targetY, 0.12);
    }

    // Обновление физических кругов ряби
    for (let i = ripples.length - 1; i >= 0; i--) {
      const r = ripples[i];
      r.radius += 5.5;
      r.alpha *= 0.965;
      if (r.alpha < 0.01 || r.radius > r.maxRadius) {
        ripples.splice(i, 1);
      }
    }

    updateColorTransition();

    ctx.clearRect(0, 0, width, height);

    // 1. Верхнее рассеянное дыхание (Ambient Radial Aura)
    const auraY = height * 0.85;
    const auraRad = Math.max(width * 0.6, 450);
    const auraGrad = ctx.createRadialGradient(width * 0.5, auraY, 40, width * 0.5, auraY, auraRad);
    auraGrad.addColorStop(0, toRgba(curColors.glow, 0.6));
    auraGrad.addColorStop(0.5, toRgba(curColors.c2, 0.25));
    auraGrad.addColorStop(1, "rgba(0,0,0,0)");
    ctx.fillStyle = auraGrad;
    ctx.fillRect(0, 0, width, height);

    // 2. Третий глубокий фоновый слой волны
    drawWave(
      height * 0.82,
      28,
      0.0035,
      0.75,
      toRgba(curColors.c3, 0.55),
      toRgba(curColors.c1, 0.25),
      0
    );

    // 3. Второй средний слой волны
    drawWave(
      height * 0.86,
      22,
      0.0048,
      -0.95,
      toRgba(curColors.c2, 0.70),
      toRgba(curColors.c3, 0.35),
      2.1
    );

    // 4. Первый фронтальный стеклянный слой волны
    drawWave(
      height * 0.90,
      16,
      0.0062,
      1.2,
      toRgba(curColors.c1, 0.85),
      toRgba(curColors.c2, 0.45),
      4.2
    );

    animFrameId = requestAnimationFrame(render);
  }

  function startAnimation() {
    if (!animFrameId) {
      animFrameId = requestAnimationFrame(render);
    }
  }

  /**
   * Смена цветового состояния (blue / amber / green / red)
   * @param {string} mood - Цветовой режим
   * @param {number} durationMs - Возврат в blue через миллисекунды (0 = навсегда)
   */
  window.setAmbientMood = function (mood, durationMs = 0) {
    if (!COLOR_PALETTES[mood]) return;
    currentMood = mood;
    if (moodReturnTimer) {
      clearTimeout(moodReturnTimer);
      moodReturnTimer = null;
    }
    if (durationMs > 0 && mood !== "blue") {
      moodReturnTimer = setTimeout(() => {
        currentMood = "blue";
      }, durationMs);
    }
  };

  /**
   * Запуск волны/импульса из указанной точки
   */
  window.triggerAmbientRipple = function (x, y, power = 1.0) {
    triggerRipple(x, y, power);
  };

  window.addEventListener("DOMContentLoaded", initAmbientFluid);
})();
