<script>
  import { onMount } from 'svelte';

  let host;
  let failed = $state(false);

  onMount(() => {
    let stop = () => {};
    Promise.all([
      import('three'),
      import('three/addons/geometries/RoundedBoxGeometry.js'),
      import('three/addons/environments/RoomEnvironment.js'),
      import('three/addons/objects/Reflector.js'),
      import('three/addons/lights/RectAreaLightUniformsLib.js'),
      import('three/addons/loaders/GLTFLoader.js')
    ])
      .then(async ([THREE, g, e, r, l, gl]) => {
        // Optional: a locally supplied laptop model (see site/static/preview). Absent in production.
        let model = null;
        try {
          if ((await fetch('/preview/mac-noUv.glb', { method: 'HEAD' })).ok) {
            model = (await new gl.GLTFLoader().loadAsync('/preview/mac-noUv.glb')).scene;
          }
        } catch {
          model = null;
        }
        stop = start(THREE, g.RoundedBoxGeometry, e.RoomEnvironment, r.Reflector, l.RectAreaLightUniformsLib, model);
      })
      .catch(() => (failed = true));
    return () => stop();
  });

  function start(THREE, RoundedBoxGeometry, RoomEnvironment, Reflector, RectAreaLightUniformsLib, model) {
    const reduce = matchMedia('(prefers-reduced-motion: reduce)').matches;
    let renderer;
    try {
      renderer = new THREE.WebGLRenderer({ antialias: true, alpha: true, powerPreference: 'high-performance' });
    } catch {
      failed = true;
      return () => {};
    }
    const PR = Math.min(devicePixelRatio, 2);
    renderer.setPixelRatio(PR);
    renderer.outputColorSpace = THREE.SRGBColorSpace;
    renderer.toneMapping = THREE.ACESFilmicToneMapping;
    renderer.toneMappingExposure = 1.0;
    renderer.shadowMap.enabled = true;
    renderer.shadowMap.type = THREE.PCFShadowMap;
    host.appendChild(renderer.domElement);
    renderer.domElement.style.cssText = 'display:block;width:100%;height:100%';

    RectAreaLightUniformsLib.init();
    const scene = new THREE.Scene();
    scene.background = new THREE.Color('#0e1013');
    const pmrem = new THREE.PMREMGenerator(renderer);
    scene.environment = pmrem.fromScene(new RoomEnvironment(), 0.03).texture;
    scene.environmentIntensity = 0.5;

    const camera = new THREE.PerspectiveCamera(26, 1, 0.1, 100);
    const root = new THREE.Group();
    scene.add(root);

    // ---------- lighting: a large softbox above-left, a hard key for shadows, a cool rim from behind
    const softbox = new THREE.RectAreaLight('#ffffff', 5.5, 7, 3);
    softbox.position.set(-3, 6.5, 3.5);
    softbox.lookAt(0, 0, 0);
    scene.add(softbox);
    const key = new THREE.DirectionalLight('#ffffff', 1.6);
    key.position.set(3.5, 8, 4.5);
    key.castShadow = true;
    key.shadow.mapSize.set(2048, 2048);
    key.shadow.radius = 8;
    key.shadow.bias = -0.0003;
    Object.assign(key.shadow.camera, { left: -9, right: 9, top: 6, bottom: -6, near: 1, far: 24 });
    scene.add(key);
    const rim = new THREE.DirectionalLight('#6f93ff', 1.4);
    rim.position.set(0, 3, -7);
    scene.add(rim);

    // ---------- floor: a faint mirror that fades into the page, plus contact shadows
    const BG = '#0e1013';
    const mirror = new Reflector(new THREE.PlaneGeometry(18, 10), {
      color: 0x6a707c,
      textureWidth: Math.floor(1024 * PR),
      textureHeight: Math.floor(1024 * PR),
      clipBias: 0.003
    });
    mirror.rotation.x = -Math.PI / 2;
    mirror.position.set(0.5, -0.002, 0.2);
    root.add(mirror);

    // an alpha mask: clear under the devices, solid page colour further out
    const fadeMask = (() => {
      const c = document.createElement('canvas');
      c.width = c.height = 512;
      const x = c.getContext('2d');
      const g = x.createRadialGradient(256, 256, 0, 256, 256, 256);
      g.addColorStop(0, '#8c8c8c');
      g.addColorStop(0.5, '#d9d9d9');
      g.addColorStop(0.85, '#ffffff');
      g.addColorStop(1, '#ffffff');
      x.fillStyle = g;
      x.fillRect(0, 0, 512, 512);
      return new THREE.CanvasTexture(c);
    })();
    const fade = new THREE.Mesh(
      new THREE.PlaneGeometry(18.02, 10.02),
      new THREE.MeshBasicMaterial({ color: BG, alphaMap: fadeMask, transparent: true, toneMapped: false, polygonOffset: true, polygonOffsetFactor: -2, polygonOffsetUnits: -2 })
    );
    fade.rotation.x = -Math.PI / 2;
    fade.position.set(0.5, 0.0005, 0.2);
    fade.renderOrder = 2;
    root.add(fade);

    const shadowFloor = new THREE.Mesh(new THREE.PlaneGeometry(40, 40), new THREE.ShadowMaterial({ opacity: 0.6 }));
    shadowFloor.rotation.x = -Math.PI / 2;
    shadowFloor.position.y = 0.001;
    shadowFloor.receiveShadow = true;
    root.add(shadowFloor);

    const glowTex = (() => {
      const c = document.createElement('canvas');
      c.width = c.height = 256;
      const x = c.getContext('2d');
      const g = x.createRadialGradient(128, 128, 0, 128, 128, 128);
      g.addColorStop(0, 'rgba(70,110,255,0.5)');
      g.addColorStop(0.55, 'rgba(110,80,255,0.14)');
      g.addColorStop(1, 'rgba(0,0,0,0)');
      x.fillStyle = g;
      x.fillRect(0, 0, 256, 256);
      const t = new THREE.CanvasTexture(c);
      t.colorSpace = THREE.SRGBColorSpace;
      return t;
    })();
    const glow = new THREE.Mesh(
      new THREE.PlaneGeometry(16, 7),
      new THREE.MeshBasicMaterial({ map: glowTex, transparent: true, depthWrite: false, toneMapped: false, blending: THREE.AdditiveBlending })
    );
    glow.rotation.x = -Math.PI / 2;
    glow.position.set(0.5, 0.003, 0.2);
    root.add(glow);

    // ---------- the extended desktop shared by both screens
    const MAC = { w: 1280, h: model ? 871 : 800 };
    const TAB = { x: 1400, w: 800, h: 500 };
    const TOTAL = TAB.x + TAB.w;
    const canvasOf = (w, h) => Object.assign(document.createElement('canvas'), { width: w, height: h });
    const macCanvas = canvasOf(1280, MAC.h);
    const tabCanvas = canvasOf(1024, 640);
    const macCtx = macCanvas.getContext('2d');
    const tabCtx = tabCanvas.getContext('2d');
    const tex = (c, aniso = 8) => {
      const t = new THREE.CanvasTexture(c);
      t.colorSpace = THREE.SRGBColorSpace;
      t.anisotropy = aniso;
      return t;
    };
    const macTex = tex(macCanvas);
    const tabTex = tex(tabCanvas);

    function wallpaper(ctx, h) {
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

    function windowAt(ctx, x, y, w, h) {
      ctx.save();
      ctx.shadowColor = 'rgba(0,0,0,0.5)';
      ctx.shadowBlur = 40;
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
      macCtx.setTransform(1, 0, 0, 1, 0, 0);
      macCtx.clearRect(0, 0, MAC.w, MAC.h);
      wallpaper(macCtx, MAC.h);
      macCtx.fillStyle = 'rgba(8,10,14,0.45)';
      macCtx.fillRect(0, 0, MAC.w, 34);
      macCtx.fillStyle = '#e8ebf0';
      macCtx.fillRect(22, 11, 12, 13);
      macCtx.fillStyle = 'rgba(232,235,240,0.55)';
      for (let i = 0; i < 4; i++) macCtx.fillRect(56 + i * 62, 14, 40, 8);
      macCtx.fillRect(MAC.w - 150, 14, 110, 8);
      macCtx.fillStyle = 'rgba(255,255,255,0.14)';
      macCtx.beginPath();
      macCtx.roundRect(MAC.w / 2 - 230, MAC.h - 88, 460, 70, 22);
      macCtx.fill();
      ['#4fd1c5', '#6e9bff', '#f5b73b', '#ff6b6b', '#b69cff', '#4ade80', '#8c95a3'].forEach((c, i) => {
        macCtx.fillStyle = c;
        macCtx.beginPath();
        macCtx.roundRect(MAC.w / 2 - 214 + i * 62, MAC.h - 78, 50, 50, 13);
        macCtx.fill();
      });
      windowAt(macCtx, wx, WIN.y, WIN.w, WIN.h);

      tabCtx.setTransform(1024 / TAB.w, 0, 0, 640 / TAB.h, 0, 0);
      tabCtx.clearRect(0, 0, TAB.w, TAB.h);
      tabCtx.translate(-TAB.x, 0);
      wallpaper(tabCtx, TAB.h);
      windowAt(tabCtx, wx, WIN.y, WIN.w, WIN.h);

      if (grabbed) {
        const cx = wx + 300;
        const cy = WIN.y + 22;
        if (cx < MAC.w) cursor(macCtx, cx, cy);
        else cursor(tabCtx, cx, cy);
      }
      macTex.needsUpdate = true;
      tabTex.needsUpdate = true;
    }

    // ---------- materials
    const noise = (() => {
      // fine horizontal grain: the brushed finish of anodised aluminium
      const c = canvasOf(512, 512);
      const x = c.getContext('2d');
      const img = x.createImageData(512, 512);
      for (let j = 0; j < 512; j++) {
        const row = 120 + Math.random() * 40;
        for (let i = 0; i < 512; i++) {
          const v = row + (Math.random() - 0.5) * 30;
          const k = (j * 512 + i) * 4;
          img.data[k] = img.data[k + 1] = img.data[k + 2] = v;
          img.data[k + 3] = 255;
        }
      }
      x.putImageData(img, 0, 0);
      const t = new THREE.CanvasTexture(c);
      t.wrapS = t.wrapT = THREE.RepeatWrapping;
      t.repeat.set(3, 3);
      return t;
    })();
    const aluminium = new THREE.MeshPhysicalMaterial({
      color: '#b8bcc4', metalness: 1, roughness: 0.42, roughnessMap: noise, clearcoat: 0.15, clearcoatRoughness: 0.4
    });
    const graphite = new THREE.MeshPhysicalMaterial({ color: '#2b2f36', metalness: 1, roughness: 0.46, roughnessMap: noise });
    const black = new THREE.MeshStandardMaterial({ color: '#07090c', roughness: 0.55, metalness: 0.1 });
    const rubber = new THREE.MeshStandardMaterial({ color: '#15181c', roughness: 0.9, metalness: 0 });
    const screenMat = (map) => new THREE.MeshBasicMaterial({ map, toneMapped: false });
    const glassOver = new THREE.MeshPhysicalMaterial({
      color: '#ffffff', transparent: true, opacity: 0.07, roughness: 0.03, metalness: 0, clearcoat: 1, clearcoatRoughness: 0.02,
      envMapIntensity: 2, depthWrite: false
    });
    const sh = (m) => ((m.castShadow = true), (m.receiveShadow = true), m);
    const box = (w, h, d, r, mat) => sh(new THREE.Mesh(new RoundedBoxGeometry(w, h, d, 4, Math.min(r, w / 2 - 0.001, h / 2 - 0.001, d / 2 - 0.001)), mat));

    // ---------- MacBook
    let mac, hinge, macScreen, macGlass, OPEN, BASE, portLocal;
    if (model) {
      // a supplied model: "_top" is the lid (hinge at its origin), "_bottom" the base
      mac = new THREE.Group();
      hinge = new THREE.Group();
      const bottom = new THREE.Group();
      mac.add(hinge, bottom);
      const logoMat = new THREE.MeshBasicMaterial({ color: '#ffffff' });
      const camMat = new THREE.MeshBasicMaterial({ color: '#333333' });
      const plastic = new THREE.MeshStandardMaterial({ color: '#050505', roughness: 0.85, metalness: 0.6 });
      for (const child of [...model.children]) {
        const top = child.name === '_top';
        (top ? hinge : bottom).add(child);
        child.traverse((m) => {
          if (!m.isMesh) return;
          m.castShadow = m.receiveShadow = true;
          m.material = { lid: aluminium, base: aluminium, logo: logoMat, camera: camMat }[m.name] ?? plastic;
        });
      }
      const SS = [29.4, 20];
      macScreen = new THREE.Mesh(new THREE.PlaneGeometry(SS[0], SS[1]), screenMat(macTex));
      macScreen.position.set(0, 10.5, -0.105);
      hinge.add(macScreen);
      macGlass = new THREE.Mesh(new THREE.PlaneGeometry(SS[0], SS[1]), glassOver);
      macGlass.position.set(0, 10.5, -0.1);
      hinge.add(macGlass);
      const keys = new THREE.Mesh(
        new THREE.PlaneGeometry(27.7, 11.6),
        new THREE.MeshBasicMaterial({ color: '#ffffff', alphaMap: new THREE.TextureLoader().load('/preview/keyboard-overlay.png'), transparent: true })
      );
      keys.rotation.x = -Math.PI / 2;
      keys.position.set(0, 0.045, 7.21);
      bottom.add(keys);
      const lb = new THREE.Box3().setFromObject(bottom);
      const sc = 4.0 / (lb.max.x - lb.min.x);
      portLocal = new THREE.Vector3(lb.max.x, lb.min.y + 0.7, (lb.max.z + lb.min.z) / 2 + 3);
      mac.scale.setScalar(sc);
      mac.position.set(-2.55 - ((lb.max.x + lb.min.x) / 2) * sc, -lb.min.y * sc, -((lb.max.z + lb.min.z) / 2) * sc);
      root.add(mac);
      OPEN = -0.2 * Math.PI;
    } else {
    mac = new THREE.Group();
    BASE = { w: 4.0, d: 2.7, h: 0.12 };
    const base = box(BASE.w, BASE.h, BASE.d, 0.055, aluminium);
    base.position.y = BASE.h / 2 + 0.02;
    mac.add(base);
    for (const [fx, fz] of [[-1.7, -1.1], [1.7, -1.1], [-1.7, 1.1], [1.7, 1.1]]) {
      const foot = new THREE.Mesh(new THREE.CylinderGeometry(0.09, 0.09, 0.02, 20), rubber);
      foot.position.set(fx, 0.01, fz);
      mac.add(foot);
    }
    const DECK_Y = BASE.h + 0.02;
    // keywell and trackpad
    const deckTex = (() => {
      const c = canvasOf(1024, 700);
      const x = c.getContext('2d');
      x.fillStyle = '#b8bcc4';
      x.fillRect(0, 0, 1024, 700);
      x.fillStyle = '#0a0c0f';
      x.beginPath();
      x.roundRect(86, 24, 852, 330, 10);
      x.fill();
      // speaker grilles
      x.fillStyle = '#5a5f68';
      for (const sx of [20, 960]) for (let j = 0; j < 24; j++) for (let i = 0; i < 3; i++) {
        x.beginPath();
        x.arc(sx + i * 12, 40 + j * 13, 2.2, 0, 7);
        x.fill();
      }
      // trackpad
      x.fillStyle = '#9fa4ad';
      x.beginPath();
      x.roundRect(330, 410, 364, 250, 12);
      x.fill();
      x.strokeStyle = 'rgba(0,0,0,0.25)';
      x.lineWidth = 2;
      x.stroke();
      return tex(c, 4);
    })();
    const deck = new THREE.Mesh(new THREE.PlaneGeometry(3.88, 2.58), new THREE.MeshPhysicalMaterial({ map: deckTex, metalness: 0.9, roughness: 0.5, roughnessMap: noise }));
    deck.rotation.x = -Math.PI / 2;
    deck.position.set(0, DECK_Y + 0.001, 0);
    deck.receiveShadow = true;
    mac.add(deck);
    // keycaps
    {
      const rows = [14, 14, 14, 13, 12];
      const kGeo = new RoundedBoxGeometry(0.2, 0.05, 0.2, 3, 0.03);
      const kMat = new THREE.MeshStandardMaterial({ color: '#16191e', roughness: 0.5, metalness: 0.2 });
      const total = rows.reduce((a, b) => a + b, 0);
      const keys = new THREE.InstancedMesh(kGeo, kMat, total);
      keys.castShadow = true;
      const m = new THREE.Matrix4();
      let n = 0;
      rows.forEach((count, r) => {
        const kw = 3.3 / count;
        for (let i = 0; i < count; i++) {
          m.makeScale(kw / 0.2 * 0.9, 1, 1);
          m.setPosition(-1.65 + kw / 2 + i * kw, DECK_Y + 0.03, -1.12 + r * 0.245);
          keys.setMatrixAt(n++, m);
        }
      });
      mac.add(keys);
      // the space bar row stays plain; add a wide space key
      const space = box(1.5, 0.05, 0.2, 0.03, kMat);
      space.position.set(0, DECK_Y + 0.03, 0.1);
      space.visible = false;
      mac.add(space);
    }
    // hinge barrel
    const barrel = new THREE.Mesh(new THREE.CylinderGeometry(0.07, 0.07, 3.3, 24), graphite);
    barrel.rotation.z = Math.PI / 2;
    barrel.position.set(0, BASE.h + 0.06, -BASE.d / 2 + 0.07);
    barrel.castShadow = true;
    mac.add(barrel);

    hinge = new THREE.Group();
    hinge.position.set(0, BASE.h + 0.07, -BASE.d / 2 + 0.07);
    mac.add(hinge);
    const LID = { w: 4.0, h: 2.6, t: 0.06 };
    const lid = box(LID.w, LID.h, LID.t, 0.03, aluminium);
    lid.position.y = LID.h / 2 + 0.02;
    hinge.add(lid);
    const bezel = new THREE.Mesh(new THREE.PlaneGeometry(LID.w - 0.1, LID.h - 0.09), black);
    bezel.position.set(0, LID.h / 2 + 0.02, LID.t / 2 + 0.002);
    hinge.add(bezel);
    macScreen = new THREE.Mesh(new THREE.PlaneGeometry(3.8, 2.375), screenMat(macTex));
    macScreen.position.set(0, LID.h / 2 + 0.02 - 0.025, LID.t / 2 + 0.004);
    hinge.add(macScreen);
    macGlass = new THREE.Mesh(new THREE.PlaneGeometry(LID.w - 0.1, LID.h - 0.09), glassOver);
    macGlass.position.set(0, LID.h / 2 + 0.02, LID.t / 2 + 0.006);
    hinge.add(macGlass);
    const notch = new THREE.Mesh(new THREE.PlaneGeometry(0.42, 0.075), black);
    notch.position.set(0, LID.h - 0.045, LID.t / 2 + 0.005);
    hinge.add(notch);
    mac.position.set(-2.55, 0, 0);
    root.add(mac);
    OPEN = -0.28;
      portLocal = new THREE.Vector3(BASE.w / 2, BASE.h / 2 + 0.02, 0.35);
    }

    // ---------- tablet on a stand
    const tabletRig = new THREE.Group(); // positioned and leaned; holds the tablet and its stand
    const T = { w: 3.1, h: 1.95, t: 0.075 };
    const tablet = new THREE.Group();
    tablet.add(box(T.w, T.h, T.t, 0.075, graphite));
    const tFace = new THREE.Mesh(new THREE.PlaneGeometry(T.w - 0.05, T.h - 0.05), black);
    tFace.position.z = T.t / 2 + 0.002;
    tablet.add(tFace);
    const tabScreen = new THREE.Mesh(new THREE.PlaneGeometry(2.96, 1.85), screenMat(tabTex));
    tabScreen.position.z = T.t / 2 + 0.004;
    tablet.add(tabScreen);
    const tabGlass = new THREE.Mesh(new THREE.PlaneGeometry(T.w - 0.05, T.h - 0.05), glassOver);
    tabGlass.position.z = T.t / 2 + 0.006;
    tablet.add(tabGlass);
    const lens = new THREE.Mesh(new THREE.CircleGeometry(0.016, 16), new THREE.MeshBasicMaterial({ color: '#0c0e12' }));
    lens.position.set(0, T.h / 2 - 0.05, T.t / 2 + 0.007);
    tablet.add(lens);
    for (const [bx, bw] of [[0.9, 0.34], [0.5, 0.2]]) {
      const b = box(bw, 0.014, 0.03, 0.007, aluminium);
      b.position.set(bx, T.h / 2 + 0.004, 0);
      tablet.add(b);
    }
    tabletRig.add(tablet);
    // stand: weighted base, a lip that holds the bottom edge, and a rear strut
    const lean = 0.26;
    const lip = box(2.0, 0.12, 0.1, 0.04, aluminium);
    lip.position.set(0, -T.h / 2 + 0.02, T.t / 2 + 0.05);
    tabletRig.add(lip);
    const strut = new THREE.Group();
    strut.position.set(0, -0.15, -T.t / 2 - 0.03);
    strut.rotation.x = 0.62;
    const strutPlate = box(1.0, 1.5, 0.045, 0.02, aluminium);
    strutPlate.position.y = -0.7;
    strut.add(strutPlate);
    tabletRig.add(strut);
    tabletRig.rotation.set(-lean, -0.3, 0, 'YXZ');
    tabletRig.position.set(3.15, (T.h / 2) * Math.cos(lean) + 0.05, 0.3);
    root.add(tabletRig);
    const standBase = box(1.9, 0.045, 1.35, 0.02, aluminium);
    standBase.position.set(3.1, 0.04, 0.0);
    standBase.rotation.y = -0.3;
    root.add(standBase);

    // ---------- USB-C cable: braided sleeve, two plugs, a natural sag on the desk
    root.updateMatrixWorld(true);
    const portMac = mac.localToWorld(portLocal.clone());
    const portTab = tablet.localToWorld(new THREE.Vector3(-T.w / 2, -0.5, 0));
    const dirTab = new THREE.Vector3(-1, 0, 0).transformDirection(tablet.matrixWorld).normalize();
    const PLUG = 0.34;
    const startPt = portMac.clone().add(new THREE.Vector3(PLUG, 0, 0));
    const endPt = portTab.clone().addScaledVector(dirTab, PLUG);
    const R = 0.034;
    const curve = new THREE.CatmullRomCurve3(
      [
        startPt,
        startPt.clone().add(new THREE.Vector3(0.28, 0, 0.05)),
        new THREE.Vector3(0.7, R, 0.95),
        new THREE.Vector3(1.55, R, 1.15),
        new THREE.Vector3(endPt.x - 0.75, R, endPt.z + 0.2),
        new THREE.Vector3(endPt.x - 0.3, endPt.y * 0.4, endPt.z + 0.02),
        endPt
      ],
      false,
      'centripetal',
      0.5
    );
    const braid = (() => {
      const c = canvasOf(64, 256);
      const x = c.getContext('2d');
      x.fillStyle = '#e9ecf1';
      x.fillRect(0, 0, 64, 256);
      x.strokeStyle = 'rgba(120,128,140,0.55)';
      x.lineWidth = 3;
      for (let i = -4; i < 14; i++) {
        x.beginPath();
        x.moveTo(0, i * 20);
        x.lineTo(64, i * 20 + 40);
        x.stroke();
      }
      const t = new THREE.CanvasTexture(c);
      t.wrapS = t.wrapT = THREE.RepeatWrapping;
      t.repeat.set(1, 60);
      t.colorSpace = THREE.SRGBColorSpace;
      return t;
    })();
    const cableMat = new THREE.MeshStandardMaterial({ color: '#f1f3f7', map: braid, roughness: 0.62, metalness: 0 });
    const TUBE_SEG = 160;
    const cable = sh(new THREE.Mesh(new THREE.TubeGeometry(curve, TUBE_SEG, R, 14), cableMat));
    root.add(cable);
    const tubeIndexCount = cable.geometry.index.count;

    function plug(position, direction) {
      const g = new THREE.Group();
      const shellMat = aluminium;
      // metal shell, 8.4 x 2.6 proportions scaled up
      const shell = box(0.2, 0.052, 0.13, 0.024, shellMat);
      shell.position.x = 0.1;
      g.add(shell);
      const tongue = box(0.08, 0.03, 0.1, 0.01, black);
      tongue.position.x = 0.06;
      g.add(tongue);
      const mold = box(0.2, 0.09, 0.17, 0.04, new THREE.MeshStandardMaterial({ color: '#f1f3f7', roughness: 0.5 }));
      mold.position.x = 0.3;
      g.add(mold);
      const relief = new THREE.Mesh(new THREE.CylinderGeometry(R * 1.25, R * 1.0, 0.16, 16), cableMat);
      relief.rotation.z = Math.PI / 2;
      relief.position.x = 0.46;
      g.add(relief);
      g.position.copy(position);
      g.quaternion.setFromUnitVectors(new THREE.Vector3(1, 0, 0), direction.clone().normalize());
      return g;
    }
    const plugMac = plug(portMac.clone().add(new THREE.Vector3(-0.04, 0, 0)), new THREE.Vector3(1, 0, 0));
    plugMac.rotation.y = 0;
    const plugTab = plug(portTab.clone().addScaledVector(dirTab, -0.05), dirTab.clone().negate());
    // orient the plug heads toward the cable
    plugMac.quaternion.setFromUnitVectors(new THREE.Vector3(1, 0, 0), new THREE.Vector3(1, 0, 0));
    root.add(plugMac, plugTab);

    // pulses of data travelling along the cable
    const pulseMat = new THREE.MeshBasicMaterial({ color: '#8fb0ff', toneMapped: false });
    const pulses = Array.from({ length: 3 }, (_, i) => {
      const m = new THREE.Mesh(new THREE.SphereGeometry(0.05, 14, 14), pulseMat);
      m.userData.o = i / 3;
      m.visible = false;
      root.add(m);
      return m;
    });

    // ---------- timeline
    const ease = (t) => (t < 0.5 ? 4 * t ** 3 : 1 - (-2 * t + 2) ** 3 / 2);
    const out = (t) => 1 - (1 - Math.min(1, Math.max(0, t))) ** 3;
    const clamp01 = (v) => Math.min(1, Math.max(0, v));
    const X0 = 130;
    const X1 = TAB.x + 110;
    const CYCLE = 12;
    const INTRO_END = 4.2;
    function windowPos(time) {
      const p = (time - INTRO_END) % CYCLE;
      if (time < INTRO_END || p < 1.5) return [X0, false];
      if (p < 4.5) return [X0 + (X1 - X0) * ease((p - 1.5) / 3), true];
      if (p < 7.5) return [X1, false];
      if (p < 10.5) return [X1 + (X0 - X1) * ease((p - 7.5) / 3), true];
      return [X0, false];
    }
    function intro(t) {
      // laptop opens, tablet slides in, cable draws, then the loop starts
      const lidK = out((t - 0.3) / 1.8);
      hinge.rotation.x = THREE.MathUtils.lerp(Math.PI / 2 - 0.02, OPEN, lidK);
      macScreen.visible = macGlass.visible = lidK > 0.55;
      const tabK = out((t - 1.0) / 1.5);
      tabletRig.position.x = 3.15 + (1 - tabK) * 2.4;
      tabletRig.position.y = (T.h / 2) * Math.cos(lean) + 0.05 + (1 - tabK) * 0.5;
      standBase.position.x = 3.1 + (1 - tabK) * 2.4;
      standBase.position.y = 0.04 + (1 - tabK) * 0.5;
      const cableK = clamp01((t - 2.4) / 1.4);
      cable.geometry.setDrawRange(0, Math.floor((tubeIndexCount * cableK) / 3) * 3);
      plugMac.visible = cableK > 0;
      plugTab.visible = cableK >= 1;
      for (const p of pulses) p.visible = cableK >= 1;
    }

    function fit() {
      const w = host.clientWidth;
      const h = host.clientHeight;
      renderer.setSize(w, h, false);
      camera.aspect = w / h;
      const half = 5.8;
      camera.userData.dist = Math.max(12.5, half / Math.tan((camera.fov * Math.PI) / 360) / Math.min(camera.aspect, 2.3));
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
    function placeCamera(t) {
      const d = camera.userData.dist * (1 + 0.12 * (1 - out(t / 3.5))); // eases in from slightly farther away
      const a = yaw + Math.sin(t * 0.25) * 0.045;
      const el = 0.34 + pitch;
      camera.position.set(Math.sin(a) * d * Math.cos(el) + 0.3, Math.sin(el) * d + 0.5, Math.cos(a) * d * Math.cos(el));
      camera.lookAt(0.35, 0.95, 0.3);
    }

    const timer = new THREE.Timer();
    let raf = 0;
    function frame() {
      raf = requestAnimationFrame(frame);
      if (!visible || document.hidden) return;
      timer.update();
      const t = timer.getElapsed();
      yaw += (pointer.x * 0.2 - yaw) * 0.05;
      pitch += (pointer.y * 0.05 - pitch) * 0.05;
      intro(t);
      placeCamera(t);
      const [wx, grabbed] = windowPos(t);
      drawDesktop(wx, grabbed);
      for (const p of pulses) curve.getPointAt((t * 0.18 + p.userData.o) % 1, p.position);
      renderer.render(scene, camera);
    }
    if (reduce) {
      intro(10);
      placeCamera(10);
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
