// Use the system Chromium when puppeteer's own Chrome download isn't
// available (thumbnail generation during the lesson build).
const fs = require('fs');

const systemChromium = ['/usr/bin/chromium', '/usr/sbin/chromium', '/usr/bin/google-chrome-stable']
  .find((p) => fs.existsSync(p));

module.exports = {
  ...(process.env.PUPPETEER_EXECUTABLE_PATH || !systemChromium
    ? {}
    : { executablePath: systemChromium }),
};
