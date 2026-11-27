<script>
  import { onMount } from 'svelte';

  let host;
  let failed = $state(false);

  onMount(() => {
    let stop = () => {};
    Promise.all([
      import('three'),
      import('three/examples/jsm/geometries/RoundedBoxGeometry.js'),
      import('three/examples/jsm/environments/RoomEnvironment.js')
    ])
      .then(([THREE, { RoundedBoxGeometry }, { RoomEnvironment }]) => {
        stop = start(THREE, RoundedBoxGeometry, RoomEnvironment);
      })
      .catch(() => (failed = true));
    return () => stop();
  });

  function start(THREE, RoundedBoxGeometry, RoomEnvironment) {
    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    let renderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'high-performance' });
    } catch {
      failed = true;
      return () => {};
    }
    renderer.setPixelRatio(Math.min(devicePixelRatio, 2));
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.05;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFSoftShadowMap;
    host.appendChild(renderer.domElement);
    renderer.domElement.style.cssText = 'display:block;width:100%;height:100%';

    const scene = new THREE.Scene();
    const pmrem = new THREE.PMREMGenerator(renderer);
    const envMap = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
    scene.environment = envMap;
    scene.environmentIntensity = 0.42;

    const camera = new THREE.PerspectiveCamera(28, 1, 0.1, 100);
    const root = new THREE.Group();
    scene.add(root);

    // ---- lights: a soft key from above-right, a cool fill from the left
    const key = new THREE.DirectionalLight('#ffffff', 2.2);
    key.position.set(4, 8, 5);
    key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    key.shadow.radius = 7;
    key.shadow.bias = -0.0004;
    Object.assign(key.shadow.camera, { left: -9, right: 9, top: 6, bottom: -6, near: 1, far: 24 });
    scene.add(key);
    const fill = new THREE.DirectionalLight('#7aa2ff', 0.6);
    fill.position.set(-6, 3, 3);
    scene.add(fill);

    // ---- floor: only receives shadows, plus a faint brand glow under the devices
    const floor = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), new THREE.ShadowMaterial({ opacity: 0.55 }));
    floor.rotation.x = -Math.PI / 2;
    floor.receiveShadow = true;
    root.add(floor);

    const glowTex = (() => {
      const c = document.createElement('canvas');
      c.width = c.height = 256;
      const x = c.getContext('2d');
      const g = x.createRadialGradient(128, 128, 0, 128, 128, 128);
      g.addColorStop(0, 'rgba(90,130,255,0.55)');
      g.addColorStop(0.5, 'rgba(110,80,255,0.18)');
      g.addColorStop(1, 'rgba(0,0,0,0)');
      x.fillStyle = g;
      x.fillRect(0, 0, 256, 256);
      const t = new THREE.CanvasTexture(c);
      t.colorSpace = THREE.SRGBColorSpace;
      return t;
    })();
    const glow = new THREE.Mesh(
      new THREE.PlaneGeometry(22, 10),
      new THREE.MeshBasicMaterial({ map: glowTex, transparent: true, depthWrite: false, toneMapped: false })
    );
    glow.rotation.x = -Math.PI / 2;
    glow.position.set(0.4, 0.002, 0.2);
    root.add(glow);

    // ---- the extended desktop: one wallpaper and one window shared by both screens
    const MAC = { w: 1280, h: 800 };
    const TAB = { x: 1400, w: 800, h: 500 }; // the tablet sits to the right of the Mac in desktop space
    const TOTAL = TAB.x + TAB.w;

    function canvasOf(w, h) {
      const c = document.createElement('canvas');
      c.width = w;
      c.height = h;
      return c;
    }
    const macCanvas = canvasOf(1280, 800);
    const tabCanvas = canvasOf(1024, 640);
    const macCtx = macCanvas.getContext('2d');
    const tabCtx = tabCanvas.getContext('2d');
    const tex = (c) => {
      const t = new THREE.CanvasTexture(c);
      t.colorSpace = THREE.SRGBColorSpace;
      t.anisotropy = 8;
      return t;
    };
    const macTex = tex(macCanvas);
    const tabTex = tex(tabCanvas);

    function wallpaper(ctx, w, h) {
      // drawn in desktop coordinates, so both screens show parts of the same picture
      const g = ctx.createLinearGradient(0, 0, TOTAL, h);
      g.addColorStop(0, '#0f1b45');
      g.addColorStop(0.5, '#1c1a63');
      g.addColorStop(1, '#3a1f78');
      ctx.fillStyle = g;
      ctx.fillRect(-10, -10, TOTAL + 20, h + 20);
      const blob = (x, y, r, c) => {
        const rg = ctx.createRadialGradient(x, y, 0, x, y, r);
        rg.addColorStop(0, c);
        rg.addColorStop(1, 'rgba(0,0,0,0)');
        ctx.fillStyle = rg;
        ctx.fillRect(x - r, y - r, r * 2, r * 2);
      };
      blob(380, 700, 520, 'rgba(18,80,211,0.55)');
      blob(1500, 120, 480, 'rgba(110,155,255,0.35)');
      blob(1950, 520, 420, 'rgba(182,100,255,0.35)');
    }

    function windowAt(ctx, x, y, w, h, shadow) {
      ctx.save();
      ctx.shadowColor = 'rgba(0,0,0,0.5)';
      ctx.shadowBlur = shadow;
      ctx.shadowOffsetY = 14;
      ctx.fillStyle = '#0e1013';
      ctx.beginPath();
      ctx.roundRect(x, y, w, h, 16);
      ctx.fill();
      ctx.restore();
      ctx.fillStyle = '#1a1f26';
      ctx.beginPath();
      ctx.roundRect(x, y, w, 40, [16, 16, 0, 0]);
      ctx.fill();
      ['#ff5f57', '#febc2e', '#28c840'].forEach((c, i) => {
        ctx.fillStyle = c;
        ctx.beginPath();
        ctx.arc(x + 24 + i * 22, y + 20, 6.5, 0, 7);
        ctx.fill();
      });
      // sidebar and code lines
      ctx.fillStyle = '#12161b';
      ctx.fillRect(x, y + 40, 120, h - 40);
      ctx.fillStyle = 'rgba(255,255,255,0.14)';
      for (let i = 0; i < 6; i++) ctx.fillRect(x + 18, y + 64 + i * 26, 60 + ((i * 29) % 36), 8);
      const colors = ['#6e9bff', '#b69cff', '#4fd1c5', '#8c95a3', '#6e9bff', '#8c95a3', '#b69cff'];
      for (let i = 0; i < 9; i++) {
        ctx.fillStyle = colors[i % colors.length];
        ctx.globalAlpha = 0.85;
        ctx.fillRect(x + 148 + (i % 3 === 2 ? 28 : 0), y + 66 + i * 26, 70 + ((i * 53) % 170), 9);
        ctx.globalAlpha = 1;
      }
    }

    function cursor(ctx, x, y) {
      ctx.save();
      ctx.translate(x, y);
      ctx.fillStyle = '#fff';
      ctx.strokeStyle = '#000';
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(0, 0);
      ctx.lineTo(0, 30);
      ctx.lineTo(8, 23);
      ctx.lineTo(14, 36);
      ctx.lineTo(20, 33);
      ctx.lineTo(14, 21);
      ctx.lineTo(25, 21);
      ctx.closePath();
      ctx.fill();
      ctx.stroke();
      ctx.restore();
    }

    const WIN = { w: 540, h: 340, y: 170 };
    let lastKey = '';
    function drawDesktop(wx, grabbed) {
      const key = wx.toFixed(0) + grabbed;
      if (key === lastKey) return;
      lastKey = key;

      // Mac screen
      macCtx.setTransform(1, 0, 0, 1, 0, 0);
      macCtx.clearRect(0, 0, MAC.w, MAC.h);
      wallpaper(macCtx, MAC.w, MAC.h);
      macCtx.fillStyle = 'rgba(8,10,14,0.45)';
      macCtx.fillRect(0, 0, MAC.w, 34);
      macCtx.fillStyle = '#e8ebf0';
      macCtx.fillRect(22, 11, 12, 13);
      macCtx.fillStyle = 'rgba(232,235,240,0.55)';
      for (let i = 0; i < 4; i++) macCtx.fillRect(56 + i * 62, 14, 40, 8);
      macCtx.fillRect(MAC.w - 150, 14, 110, 8);
      // dock
      macCtx.fillStyle = 'rgba(255,255,255,0.14)';
      macCtx.beginPath();
      macCtx.roundRect(MAC.w / 2 - 230, MAC.h - 88, 460, 70, 22);
      macCtx.fill();
      const dock = ['#4fd1c5', '#6e9bff', '#f5b73b', '#ff6b6b', '#b69cff', '#4ade80', '#8c95a3'];
      dock.forEach((c, i) => {
        macCtx.fillStyle = c;
        macCtx.beginPath();
        macCtx.roundRect(MAC.w / 2 - 214 + i * 62, MAC.h - 78, 50, 50, 13);
        macCtx.fill();
      });
      windowAt(macCtx, wx, WIN.y, WIN.w, WIN.h, 40);
      macTex.needsUpdate = true;

      // tablet screen: same desktop, shifted
      tabCtx.setTransform(1024 / TAB.w, 0, 0, 640 / TAB.h, 0, 0);
      tabCtx.clearRect(0, 0, TAB.w, TAB.h);
      tabCtx.translate(-TAB.x, 0);
      wallpaper(tabCtx, TAB.w, TAB.h);
      windowAt(tabCtx, wx, WIN.y, WIN.w, WIN.h, 40);
      tabTex.needsUpdate = true;

      // the pointer holds the window by its title bar
      if (grabbed) {
        const cx = wx + 300;
        const cy = WIN.y + 22;
        if (cx < MAC.w) cursor(macCtx, cx, cy);
        else {
          tabCtx.setTransform(1024 / TAB.w, 0, 0, 640 / TAB.h, 0, 0);
          tabCtx.translate(-TAB.x, 0);
          cursor(tabCtx, cx, cy);
        }
        macTex.needsUpdate = true;
        tabTex.needsUpdate = true;
      }
    }

    // ---- materials
    const aluminium = new THREE.MeshStandardMaterial({ color: '#b4b9c1', metalness: 1, roughness: 0.38 });
    const graphite = new THREE.MeshStandardMaterial({ color: '#2a2e34', metalness: 0.9, roughness: 0.4 });
    const glass = new THREE.MeshStandardMaterial({ color: '#05070a', metalness: 0, roughness: 0.25 });
    const screenMat = (map) => new THREE.MeshBasicMaterial({ map, toneMapped: false });

    function shadowed(mesh) {
      mesh.castShadow = true;
      mesh.receiveShadow = true;
      return mesh;
    }

    // ---- MacBook
    const mac = new THREE.Group();
    const BASE = { w: 4.0, d: 2.7, h: 0.13 };
    const base = shadowed(new THREE.Mesh(new RoundedBoxGeometry(BASE.w, BASE.h, BASE.d, 5, 0.06), aluminium));
    base.position.y = BASE.h / 2;
    mac.add(base);

    const deckTex = (() => {
      const c = canvasOf(1024, 700);
      const x = c.getContext('2d');
      x.fillStyle = '#16191e';
      x.fillRect(0, 0, 1024, 700);
      x.fillStyle = '#272c33';
      const rows = [14, 14, 14, 13, 12];
      let y = 40;
      rows.forEach((n, r) => {
        const kw = 940 / n;
        for (let i = 0; i < n; i++) {
          x.beginPath();
          x.roundRect(42 + i * kw, y, kw - 8, 62, 8);
          x.fill();
        }
        y += 76;
      });
      x.fillStyle = '#1f2329';
      x.beginPath();
      x.roundRect(330, 450, 364, 220, 14);
      x.fill();
      return tex(c);
    })();
    const deck = new THREE.Mesh(new THREE.PlaneGeometry(3.7, 2.2), new THREE.MeshStandardMaterial({ map: deckTex, roughness: 0.6, metalness: 0.2 }));
    deck.rotation.x = -Math.PI / 2;
    deck.position.set(0, BASE.h + 0.002, -0.12);
    deck.receiveShadow = true;
    mac.add(deck);

    const hinge = new THREE.Group();
    hinge.position.set(0, BASE.h, -BASE.d / 2 + 0.06);
    hinge.rotation.x = -0.27; // open a little past vertical
    mac.add(hinge);
    const LID = { w: 4.0, h: 2.6, t: 0.075 };
    const lid = shadowed(new THREE.Mesh(new RoundedBoxGeometry(LID.w, LID.h, LID.t, 5, 0.035), aluminium));
    lid.position.y = LID.h / 2 + 0.02;
    hinge.add(lid);
    const bezel = new THREE.Mesh(new THREE.PlaneGeometry(LID.w - 0.1, LID.h - 0.1), glass);
    bezel.position.set(0, LID.h / 2 + 0.02, LID.t / 2 + 0.002);
    hinge.add(bezel);
    const macScreen = new THREE.Mesh(new THREE.PlaneGeometry(3.76, 2.35), screenMat(macTex));
    macScreen.position.set(0, LID.h / 2 + 0.02 - 0.03, LID.t / 2 + 0.004);
    hinge.add(macScreen);
    mac.position.set(-2.5, 0, 0);
    root.add(mac);

    // ---- tablet on a small stand
    const tablet = new THREE.Group();
    const T = { w: 3.1, h: 1.95, t: 0.11 };
    const body = shadowed(new THREE.Mesh(new RoundedBoxGeometry(T.w, T.h, T.t, 5, 0.1), graphite));
    tablet.add(body);
    const tFace = new THREE.Mesh(new THREE.PlaneGeometry(T.w - 0.06, T.h - 0.06), glass);
    tFace.position.z = T.t / 2 + 0.002;
    tablet.add(tFace);
    const tabScreen = new THREE.Mesh(new THREE.PlaneGeometry(2.92, 1.826), screenMat(tabTex));
    tabScreen.position.z = T.t / 2 + 0.004;
    tablet.add(tabScreen);
    const lens = new THREE.Mesh(new THREE.CircleGeometry(0.018, 16), new THREE.MeshBasicMaterial({ color: '#101318' }));
    lens.position.set(0, T.h / 2 - 0.058, T.t / 2 + 0.006);
    tablet.add(lens);
    const lean = 0.24;
    tablet.rotation.set(-lean, -0.32, 0, 'YXZ');
    tablet.position.set(3.0, (T.h / 2) * Math.cos(lean) + 0.03, 0.25);
    root.add(tablet);
    const leg = shadowed(new THREE.Mesh(new RoundedBoxGeometry(0.9, 1.35, 0.05, 4, 0.02), aluminium));
    leg.rotation.set(0.5, -0.32, 0, 'YXZ');
    leg.position.set(3.18, 0.62, -0.42);
    root.add(leg);

    // ---- cable and the data running through it
    const cableMat = new THREE.MeshStandardMaterial({ color: '#e9ecf1', roughness: 0.55, metalness: 0 });
    const curve = new THREE.CatmullRomCurve3([
      new THREE.Vector3(-0.52, 0.1, -0.2),
      new THREE.Vector3(0.1, 0.03, 0.5),
      new THREE.Vector3(1.0, 0.03, 1.15),
      new THREE.Vector3(1.9, 0.03, 0.85),
      new THREE.Vector3(2.45, 0.1, 0.1)
    ]);
    root.add(shadowed(new THREE.Mesh(new THREE.TubeGeometry(curve, 80, 0.028, 10), cableMat)));
    const pulses = Array.from({ length: 4 }, (_, i) => {
      const m = new THREE.Mesh(new THREE.SphereGeometry(0.045, 14, 14), new THREE.MeshBasicMaterial({ color: '#6e9bff', toneMapped: false }));
      m.userData.o = i / 4;
      root.add(m);
      return m;
    });

    // ---- the window travels: hold, drag to the tablet, hold, drag back
    const ease = (t) => (t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2);
    const X0 = 130;
    const X1 = TAB.x + 110;
    const CYCLE = 12;
    function windowPos(time) {
      const p = time % CYCLE;
      if (p < 1.5) return [X0, false];
      if (p < 4.5) return [X0 + (X1 - X0) * ease((p - 1.5) / 3), true];
      if (p < 7.5) return [X1, false];
      if (p < 10.5) return [X1 + (X0 - X1) * ease((p - 7.5) / 3), true];
      return [X0, false];
    }

    function fit() {
      const w = host.clientWidth;
      const h = host.clientHeight;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      const half = 5.5;
      const dist = Math.max(11.8, half / Math.tan((camera.fov * Math.PI) / 360) / Math.min(camera.aspect, 2.3));
      camera.userData.dist = dist;
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

    let yaw = 0;
    let pitch = 0;
    function placeCamera() {
      const d = camera.userData.dist;
      const a = yaw;
      const el = 0.36 + pitch; // looking down at about 20 degrees
      camera.position.set(Math.sin(a) * d * Math.cos(el) + 0.3, Math.sin(el) * d + 0.4, Math.cos(a) * d * Math.cos(el));
      camera.lookAt(0.3, 1.0, 0.2);
    }

    const clock = new THREE.Clock();
    let raf = 0;
    function frame() {
      raf = requestAnimationFrame(frame);
      if (!visible || document.hidden) return;
      const t = clock.getElapsedTime();
      yaw += (pointer.x * 0.22 - yaw) * 0.05;
      pitch += (pointer.y * 0.06 - pitch) * 0.05;
      placeCamera();
      const [wx, grabbed] = windowPos(t);
      drawDesktop(wx, grabbed);
      for (const p of pulses) curve.getPointAt((t * 0.2 + p.userData.o) % 1, p.position);
      renderer.render(scene, camera);
    }
    if (reduce) {
      placeCamera();
      drawDesktop(X1, false);
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
      pmrem.dispose();
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
