<script>
  import { onMount } from 'svelte';

  let host;
  let failed = $state(false);

  onMount(() => {
    let stop = () => {};
    import('three')
      .then((THREE) => {
        stop = start(THREE);
      })
      .catch(() => (failed = true));
    return () => stop();
  });

  function start(THREE) {
    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    let renderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true });
    } catch {
      failed = true;
      return () => {};
    }
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    host.appendChild(renderer.domElement);
    renderer.domElement.style.cssText = 'display:block;width:100%;height:100%';

    const scene = new THREE.Scene();
    const camera = new THREE.PerspectiveCamera(32, 1, 0.1, 100);
    const root = new THREE.Group();
    scene.add(root);

    const ACCENT = '#6e9bff';
    const flat = (color, opacity = 1) =>
      new THREE.MeshBasicMaterial({ color, transparent: opacity < 1, opacity });

    function rounded(w, h, r) {
      const s = new THREE.Shape();
      s.moveTo(-w / 2 + r, -h / 2);
      s.lineTo(w / 2 - r, -h / 2);
      s.quadraticCurveTo(w / 2, -h / 2, w / 2, -h / 2 + r);
      s.lineTo(w / 2, h / 2 - r);
      s.quadraticCurveTo(w / 2, h / 2, w / 2 - r, h / 2);
      s.lineTo(-w / 2 + r, h / 2);
      s.quadraticCurveTo(-w / 2, h / 2, -w / 2, h / 2 - r);
      s.lineTo(-w / 2, -h / 2 + r);
      s.quadraticCurveTo(-w / 2, -h / 2, -w / 2 + r, -h / 2);
      return new THREE.ShapeGeometry(s, 8);
    }

    function canvasTexture(w, h, draw) {
      const c = document.createElement('canvas');
      c.width = w;
      c.height = h;
      draw(c.getContext('2d'), w, h);
      const t = new THREE.CanvasTexture(c);
      t.colorSpace = THREE.SRGBColorSpace;
      t.anisotropy = 4;
      return t;
    }

    const wallpaper = (ctx, w, h, dim = 1) => {
      const g = ctx.createLinearGradient(0, 0, w, h);
      g.addColorStop(0, '#16224a');
      g.addColorStop(0.55, '#1a1a52');
      g.addColorStop(1, '#2a1a5e');
      ctx.fillStyle = g;
      ctx.fillRect(0, 0, w, h);
      if (dim < 1) {
        ctx.fillStyle = `rgba(8,10,14,${1 - dim})`;
        ctx.fillRect(0, 0, w, h);
      }
    };

    // A display: bezel + screen (+ optional stand).
    function display({ w, h, bezel, radius, tex, stand }) {
      const g = new THREE.Group();
      const body = new THREE.Mesh(rounded(w + bezel * 2, h + bezel * 2, radius), flat('#1b2128'));
      g.add(body);
      const screen = new THREE.Mesh(new THREE.PlaneGeometry(w, h), new THREE.MeshBasicMaterial({ map: tex }));
      screen.position.z = 0.01;
      g.add(screen);
      if (stand) {
        const neck = new THREE.Mesh(new THREE.PlaneGeometry(0.5, 0.9), flat('#161b21'));
        neck.position.set(0, -(h / 2 + bezel + 0.4), -0.01);
        g.add(neck);
        const foot = new THREE.Mesh(rounded(2.2, 0.16, 0.08), flat('#1b2128'));
        foot.position.set(0, -(h / 2 + bezel + 0.88), -0.01);
        g.add(foot);
      }
      return g;
    }

    const macTex = canvasTexture(1280, 800, (ctx, w, h) => {
      wallpaper(ctx, w, h);
      ctx.fillStyle = 'rgba(8,10,14,0.55)';
      ctx.fillRect(0, 0, w, 34);
      ctx.fillStyle = '#e8ebf0';
      ctx.fillRect(18, 11, 12, 12);
      ctx.fillStyle = 'rgba(232,235,240,0.5)';
      for (let i = 0; i < 4; i++) ctx.fillRect(48 + i * 56, 13, 36, 8);
      ctx.fillRect(w - 130, 13, 100, 8);
      // a static window
      ctx.fillStyle = 'rgba(14,16,20,0.92)';
      ctx.beginPath();
      ctx.roundRect(70, 120, 560, 380, 14);
      ctx.fill();
      ctx.fillStyle = 'rgba(255,255,255,0.12)';
      for (let i = 0; i < 9; i++) ctx.fillRect(100, 170 + i * 34, 120 + ((i * 97) % 340), 8);
    });
    const tabTex = canvasTexture(1280, 800, (ctx, w, h) => {
      wallpaper(ctx, w, h, 0.9);
      ctx.fillStyle = 'rgba(255,255,255,0.1)';
      ctx.beginPath();
      ctx.roundRect(120, 560, 360, 120, 14);
      ctx.fill();
    });

    const mac = display({ w: 6.0, h: 3.75, bezel: 0.1, radius: 0.16, tex: macTex, stand: true });
    mac.position.set(-2.35, 0.5, 0);
    root.add(mac);

    const tab = display({ w: 3.2, h: 2.0, bezel: 0.09, radius: 0.22, tex: tabTex, stand: false });
    tab.position.set(3.35, 0.05, 0.2);
    tab.rotation.y = -0.3;
    root.add(tab);

    // The cable, with pulses running along it.
    const curve = new THREE.CatmullRomCurve3([
      new THREE.Vector3(-1.45, -2.46, 0),
      new THREE.Vector3(0.2, -2.85, 0.3),
      new THREE.Vector3(2.0, -2.2, 0.5),
      new THREE.Vector3(3.0, -1.1, 0.25)
    ]);
    root.add(new THREE.Mesh(new THREE.TubeGeometry(curve, 48, 0.025, 6), flat('#2a323c')));
    const pulses = Array.from({ length: 5 }, (_, i) => {
      const m = new THREE.Mesh(new THREE.SphereGeometry(0.05, 12, 12), flat(ACCENT));
      m.userData.o = i / 5;
      root.add(m);
      return m;
    });

    // The window that travels from the Mac to the tablet.
    const winTex = canvasTexture(640, 400, (ctx, w, h) => {
      ctx.fillStyle = '#0e1013';
      ctx.beginPath();
      ctx.roundRect(0, 0, w, h, 22);
      ctx.fill();
      ctx.fillStyle = '#1b2027';
      ctx.fillRect(0, 0, w, 48);
      ['#ff5f57', '#febc2e', '#28c840'].forEach((c, i) => {
        ctx.fillStyle = c;
        ctx.beginPath();
        ctx.arc(30 + i * 28, 24, 8, 0, 7);
        ctx.fill();
      });
      ctx.fillStyle = ACCENT;
      ctx.fillRect(40, 96, 150, 12);
      ctx.fillStyle = 'rgba(255,255,255,0.22)';
      [380, 300, 440, 260, 360, 200].forEach((len, i) => ctx.fillRect(40, 140 + i * 38, len, 10));
    });
    const win = new THREE.Mesh(
      new THREE.PlaneGeometry(1.7, 1.06),
      new THREE.MeshBasicMaterial({ map: winTex, transparent: true })
    );
    root.add(win);

    const A = new THREE.Vector3(-3.55, 0.55, 0.03);
    const B = new THREE.Vector3(3.3, 0.0, 0.34);
    const lift = 0.9;
    const ease = (t) => (t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2);
    const CYCLE = 9;

    function place(time) {
      const p = time % CYCLE;
      let k = 0;
      let alpha = 1;
      if (p < 1.4) {
        alpha = Math.min(1, p / 0.5);
      } else if (p < 4.2) {
        k = ease((p - 1.4) / 2.8);
      } else if (p < 7.6) {
        k = 1;
      } else {
        k = 1;
        alpha = Math.max(0, 1 - (p - 7.6) / 1.0);
      }
      win.position.lerpVectors(A, B, k);
      win.position.y += Math.sin(k * Math.PI) * lift;
      win.position.z += Math.sin(k * Math.PI) * 0.9;
      win.rotation.y = -0.3 * k;
      const s = 1 - 0.3 * k;
      win.scale.set(s, s, 1);
      win.material.opacity = alpha;
    }

    function fit() {
      const w = host.clientWidth;
      const h = host.clientHeight;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      // Keep the whole scene (about 10.5 units wide) in view on narrow screens.
      const half = 5.6;
      const dist = Math.max(11, half / Math.tan((camera.fov * Math.PI) / 360) / Math.min(camera.aspect, 2.2));
      camera.position.set(0, 0.2, dist);
      camera.lookAt(0, 0.1, 0);
      camera.updateProjectionMatrix();
    }
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(host);

    const pointer = { x: 0, y: 0 };
    const onMove = (e) => {
      const r = host.getBoundingClientRect();
      pointer.x = ((e.clientX - r.left) / r.width - 0.5) * 2;
      pointer.y = ((e.clientY - r.top) / r.height - 0.5) * 2;
    };
    if (!reduce) addEventListener('pointermove', onMove);

    let visible = true;
    const io = new IntersectionObserver(([e]) => (visible = e.isIntersecting));
    io.observe(host);

    const clock = new THREE.Clock();
    let raf = 0;
    let rx = 0;
    let ry = 0;
    function frame() {
      raf = requestAnimationFrame(frame);
      if (!visible || document.hidden) return;
      const t = clock.getElapsedTime();
      ry += (pointer.x * 0.1 - ry) * 0.05;
      rx += (-pointer.y * 0.05 - rx) * 0.05;
      root.rotation.y = ry + Math.sin(t * 0.3) * 0.03;
      root.rotation.x = rx;
      place(t);
      for (const p of pulses) curve.getPointAt((t * 0.22 + p.userData.o) % 1, p.position);
      renderer.render(scene, camera);
    }
    if (reduce) {
      place(6); // the window has landed on the tablet
      for (const p of pulses) curve.getPointAt(p.userData.o, p.position);
      renderer.render(scene, camera);
    } else {
      frame();
    }

    return () => {
      cancelAnimationFrame(raf);
      ro.disconnect();
      io.disconnect();
      removeEventListener('pointermove', onMove);
      renderer.dispose();
      renderer.domElement.remove();
    };
  }
</script>

<div class="scene" bind:this={host} aria-hidden="true">
  {#if failed}<img src="/info-screen.png" alt="" />{/if}
</div>

<style>
  .scene { width: 100%; aspect-ratio: 16 / 9; min-height: 280px; }
  .scene img { width: 100%; height: 100%; object-fit: contain; border-radius: 14px; }
</style>
