export const SITE = 'https://ligarview.rizkykurniawan.id';
export const NAME = 'Ligarview';
export const REPO = 'https://github.com/kykurniawan/ligarview';
export const OG_IMAGE = `${SITE}/og.png`;
export const OG_ALT = 'Ligarview shows the Mac desktop on an Android tablet used as a second display.';

/** Every public page: path, title, description. The sitemap and the docs pages read this. */
export const PAGES = [
  {
    path: '/',
    title: 'Ligarview: use an Android tablet as a second Mac display',
    description:
      'Ligarview turns an Android tablet into a real extended display for your Mac over USB or Wi-Fi. Open source, built in Rust and Kotlin, no USB debugging needed.'
  },
  {
    path: '/download',
    title: 'Download Ligarview for Mac and Android',
    description:
      'Download the Ligarview Mac app (Apple Silicon) and the Android app. Free and open source. Use your Android tablet as a second Mac display over USB or Wi-Fi.'
  },
  {
    path: '/docs',
    title: 'Ligarview documentation',
    description:
      'Documentation for Ligarview: use an Android tablet as an extended second display for your Mac. Install, connect, settings, troubleshooting and how it works.'
  },
  {
    path: '/docs/install',
    title: 'Install Ligarview on Mac and Android',
    description:
      'Install the Ligarview Mac app from the disk image and the Android app from the APK, including the one-time macOS and Android security prompts.'
  },
  {
    path: '/docs/connect',
    title: 'Connect an Android tablet to your Mac over USB or Wi-Fi',
    description:
      'Connect the tablet over a USB cable without USB debugging, or pair it once over Wi-Fi with a six-digit code and let it reconnect by itself.'
  },
  {
    path: '/docs/host-app',
    title: 'The Ligarview Mac app: settings, sharing and pairing',
    description:
      'What the Ligarview Mac window does: start and stop sharing, pair or unpair a tablet, and choose orientation, frame rate, quality, codec and start at login.'
  },
  {
    path: '/docs/android-app',
    title: 'The Ligarview Android app',
    description:
      'A tour of the Ligarview Android app: link status, a live preview of the Mac screen, full screen mode, connection steps and tablet recommendations.'
  },
  {
    path: '/docs/troubleshooting',
    title: 'Troubleshooting Ligarview',
    description:
      'Fixes for a tablet that will not connect over USB or Wi-Fi, repeated Screen Recording prompts, a blurry picture, lag and a missing cursor.'
  },
  {
    path: '/docs/how-it-works',
    title: 'How Ligarview works',
    description:
      'How Ligarview builds a virtual display on macOS, captures and encodes it in hardware, and streams it to Android over USB accessory mode or TCP with a low-latency cursor.'
  }
];

export const pageFor = (/** @type {string} */ path) => {
  const clean = path.replace(/\/$/, '') || '/';
  return PAGES.find((p) => p.path === clean);
};
