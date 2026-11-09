/**
 * ambient_fluid.js - Мягкий светящийся атмосферный туман (Ambient Aura Mist)
 * Расположен строго на глубоком фоне (без курсорных фонариков и без наложений на окна)
 */

(function () {
  let canvas = null;
  let ctx = null;
  let animFrameId = null;
  let width = 0;
  let height = 0;

  // Цветовые палитры для атмосферного тумана
  // Синий (штатный режим / статика)
  // Янтарный (выполнение скрипта / загрузка / опрос)
  // Зеленый (успешное завершение действия)
  // Красный (ошибка связи / линия 0V)
  const COLOR_PALETTES = {
    blue: {
      primary: [14, 110, 230],
      secondary: [58, 70, 210],
      glow: [30, 140, 255]
    },
    amber: {
      primary: [235, 130, 15],
      secondary: [240, 175, 20],
      glow: [255, 150, 25]
    },
    green: {
      primary: [38, 175, 78],
      secondary: [30, 145, 95],
      glow: [50, 205, 90]
    },
    red: {
      primary: [220, 50, 40],
      secondary: [195, 35, 60],
      glow: [245, 65, 55]
    }
  };

  let currentMood = "blue";
  let moodReturnTimer = null;

  // Текущие интерполируемые цвета
  const curColors = {
    primary: [14, 110, 230],
    secondary: [58, 70, 210],
    glow: [30, 140, 255]
  };

  // Мягкие гармонические узлы светящегося тумана на заднем плане
  const mistNodes = [
    { xRatio: 0.22, yRatio: 0.82, radiusRatio: 0.48, speedX: 0.0005, speedY: 0.0006, phase: 0.0, alpha: 0.14 },
    { xRatio: 0.76, yRatio: 0.85, radiusRatio: 0.52, speedX: -0.0006, speedY: 0.0004, phase: 1.8, alpha: 0.12 },
    { xRatio: 0.46, yRatio: 0.68, radiusRatio: 0.42, speedX: 0.0004, speedY: -0.0005, phase: 3.5, alpha: 0.10 },
    { xRatio: 0.90, yRatio: 0.62, radiusRatio: 0.38, speedX: -0.0003, speedY: 0.0005, phase: 4.9, alpha: 0.09 }
  ];

  // Мягкие импульсы (риплы тумана при клике по вкладкам/кнопкам)
  const ripples = [];
  let time = 0;

  function initAmbientMist() {
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

    // Мягкий рассеивающийся импульс тумана при переключении разделов и кликах
    document.addEventListener("click", (e) => {
      const btn = e.target.closest("button, .hdmi-chip, .seg-btn, .apple-btn");
      if (btn) {
        const rect = btn.getBoundingClientRect();
        triggerRipple(rect.left + rect.width / 2, rect.top + rect.height / 2, 0.7);
      }
    }, { passive: true });

    startAnimation();
  }

  function onResize() {
    if (!canvas) return;
    const dpr = Math.min(window.devicePixelRatio || 1, 1.25);
    width = window.innerWidth;
    height = window.innerHeight;
    canvas.width = Math.floor(width * dpr);
    canvas.height = Math.floor(height * dpr);
    canvas.style.width = width + "px";
    canvas.style.height = height + "px";
    if (ctx) {
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    }
  }

  function triggerRipple(x, y, strength = 1.0) {
    if (ripples.length > 4) ripples.shift();
    ripples.push({
      x: x,
      y: y,
      radius: 30,
      maxRadius: Math.max(width, height) * 0.38,
      alpha: 0.14 * strength,
      growth: 2.8
    });
  }

  function lerp(a, b, t) {
    return a + (b - a) * t;
  }

  function updateColors() {
    const target = COLOR_PALETTES[currentMood] || COLOR_PALETTES.blue;
    const speed = 0.05; // Плавный переход оттенков
    for (let i = 0; i < 3; i++) {
      curColors.primary[i] = lerp(curColors.primary[i], target.primary[i], speed);
      curColors.secondary[i] = lerp(curColors.secondary[i], target.secondary[i], speed);
      curColors.glow[i] = lerp(curColors.glow[i], target.glow[i], speed);
    }
  }

  function drawSoftMistSpot(x, y, radius, color, maxAlpha) {
    if (radius <= 0 || maxAlpha <= 0.001) return;
    const rInt = Math.round(color[0]);
    const gInt = Math.round(color[1]);
    const bInt = Math.round(color[2]);

    const grad = ctx.createRadialGradient(x, y, 0, x, y, radius);
    grad.addColorStop(0, `rgba(${rInt}, ${gInt}, ${bInt}, ${maxAlpha})`);
    grad.addColorStop(0.35, `rgba(${rInt}, ${gInt}, ${bInt}, ${maxAlpha * 0.60})`);
    grad.addColorStop(0.70, `rgba(${rInt}, ${gInt}, ${bInt}, ${maxAlpha * 0.18})`);
    grad.addColorStop(1, `rgba(${rInt}, ${gInt}, ${bInt}, 0)`);

    ctx.fillStyle = grad;
    ctx.beginPath();
    ctx.arc(x, y, radius, 0, Math.PI * 2);
    ctx.fill();
  }

  function render() {
    if (!ctx || width === 0 || height === 0) {
      animFrameId = requestAnimationFrame(render);
      return;
    }

    time += 0.010;
    updateColors();

    ctx.clearRect(0, 0, width, height);

    // Отрисовка плавно дышащих сфер тумана глубоко на заднем плане
    mistNodes.forEach((node, idx) => {
      const offsetX = Math.sin(time * 0.6 + node.phase) * (width * 0.07);
      const offsetY = Math.cos(time * 0.5 + node.phase * 1.3) * (height * 0.04);
      const breath = 1.0 + Math.sin(time * 1.1 + node.phase) * 0.10;

      const posX = node.xRatio * width + offsetX;
      const posY = node.yRatio * height + offsetY;
      const radius = Math.min(width, height) * node.radiusRatio * breath;

      const col = idx % 2 === 0 ? curColors.primary : curColors.secondary;
      drawSoftMistSpot(posX, posY, radius, col, node.alpha);
    });

    // Мягкие расходящиеся импульсы тумана (растворяются в фоне)
    for (let i = ripples.length - 1; i >= 0; i--) {
      const rip = ripples[i];
      rip.radius += rip.growth;
      rip.alpha *= 0.96;

      if (rip.alpha > 0.003 && rip.radius < rip.maxRadius) {
        drawSoftMistSpot(rip.x, rip.y, rip.radius, curColors.glow, rip.alpha);
      } else {
        ripples.splice(i, 1);
      }
    }

    animFrameId = requestAnimationFrame(render);
  }

  function startAnimation() {
    if (!animFrameId) {
      animFrameId = requestAnimationFrame(render);
    }
  }

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

  window.triggerAmbientRipple = function (x, y, strength = 1.0) {
    triggerRipple(x, y, strength);
  };

  window.addEventListener("DOMContentLoaded", initAmbientMist);
})();
