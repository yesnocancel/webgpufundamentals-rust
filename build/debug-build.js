// Debug: run lesson-builder directly to see errors grunt swallows.
'use strict';
const fixLinks = require('./fix-links.js');
require('./register-toc-steps.js');
const buildStuff = require('@gfxfundamentals/lesson-builder');

const buildSettings = {
  outDir: 'out',
  baseUrl: 'https://webgpufundamentals.org',
  rootFolder: 'webgpu',
  lessonGrep: 'webgpu*.md',
  siteName: 'webgpufundamentals',
  siteThumbnail: 'webgpufundamentals.jpg',
  templatePath: 'build/templates',
  owner: 'gfxfundamentals',
  repo: 'webgpufundamentals',
  postHTMLFn: fixLinks,
  thumbnailOptions: {
    thumbnailBackground: 'webgpufundamentals-background.jpg',
    text: [
      {
        font: 'bold 100px lesson-font',
        verticalSpacing: 100,
        offset: [100, 120],
        textAlign: 'left',
        shadowOffset: [15, 15],
        strokeWidth: 15,
        textWrapWidth: 1000,
      },
      {
        font: 'bold 60px lesson-font',
        text: 'webgpufundamentals.org',
        verticalSpacing: 100,
        offset: [-100, -90],
        textAlign: 'right',
        shadowOffset: [8, 8],
        strokeWidth: 15,
        textWrapWidth: 1000,
        color: 'hsl(340, 100%, 70%)',
      },
    ],
  },
};

buildStuff(buildSettings)
  .then(() => console.log('BUILD OK'))
  .catch((e) => {
    console.error('BUILD FAILED:');
    console.error(e && e.stack || e);
    process.exitCode = 1;
  });
