/** @type {import('next').NextConfig} */
const nextConfig = {
  // COOP + COEP are required for SharedArrayBuffer (Emscripten with pthreads).
  // They also give a clean isolation context. All assets here are same-origin.
  async headers() {
    return [
      {
        source: '/(.*)',
        headers: [
          { key: 'Cross-Origin-Opener-Policy',   value: 'same-origin' },
          { key: 'Cross-Origin-Embedder-Policy',  value: 'require-corp' },
        ],
      },
    ];
  },
};

module.exports = nextConfig;
