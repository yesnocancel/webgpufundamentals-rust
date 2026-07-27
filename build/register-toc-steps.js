// Register the `toc-steps` Handlebars helper for build/debug-build.js.
// Upstream registers this helper in Gruntfile.js (kept verbatim there for
// merge-friendliness); debug-build.js bypasses grunt, so it needs its own
// registration. Same logic as the Gruntfile copy.
'use strict';
const path = require('path');
const fs = require('fs');
const Handlebars = require('handlebars');
const hanson = require('hanson');

Handlebars.registerHelper('toc-steps', function(options) {
  if (!options || !options.hash || !options.hash.list) {
    return '';
  }

  const listFilename = path.basename(options.hash.list);
  const lessonsDir = path.resolve(path.join(process.cwd(), 'webgpu/lessons'));
  const expectedPrefix = lessonsDir + path.sep;
  const resolvedListPath = path.resolve(lessonsDir, listFilename);

  if (!resolvedListPath.startsWith(expectedPrefix)) {
    throw new Error(`Security Error: Path traversal detected for ${resolvedListPath}`);
  }

  if (!fs.existsSync(resolvedListPath)) {
    throw new Error(`List file not found: ${resolvedListPath}`);
  }

  const listContent = fs.readFileSync(resolvedListPath, 'utf-8');
  const articleFilenames = hanson.parse(listContent);

  if (!Array.isArray(articleFilenames)) {
    throw new Error(`Expected array in hanson file: ${resolvedListPath}`);
  }

  const root = options.data && options.data.root ? options.data.root : this;
  const currentLang = root.lang || 'en';
  const currentContentFileName = root.contentFileName || '';
  const currentBasename = path.basename(currentContentFileName);
  const hereText = root.here || '(here)';

  const lis = articleFilenames.map((rawFilename) => {
    const sanitizedFilename = path.basename(rawFilename);

    // Check for localized file first, then fallback to English
    let mdPath = path.resolve(lessonsDir, currentLang === 'en' ? '' : currentLang, sanitizedFilename);
    const expectedMdPrefix = path.resolve(lessonsDir, currentLang === 'en' ? '' : currentLang) + path.sep;

    if (!mdPath.startsWith(expectedMdPrefix)) {
      throw new Error(`Security Error: Path traversal detected for ${mdPath}`);
    }

    if (!fs.existsSync(mdPath)) {
      mdPath = path.resolve(lessonsDir, sanitizedFilename);
      if (!mdPath.startsWith(expectedPrefix)) {
        throw new Error(`Security Error: Path traversal detected for ${mdPath}`);
      }
    }

    let title = sanitizedFilename;
    const htmlFilename = sanitizedFilename.replace(/\.md$/, '.html');
    let href = `${(currentLang === 'en' ? '' : '../')}${htmlFilename}`;
    if (fs.existsSync(mdPath)) {
      href = htmlFilename;
      const content = fs.readFileSync(mdPath, 'utf-8');
      const lines = content.split('\n');
      let headerTitle = '';
      let headerToc = '';
      for (const rawLine of lines) {
        const line = rawLine.trim();
        const m = /([A-Z0-9_-]+): (.*?)$/i.exec(line);
        if (!m) {
          break;
        }
        const key = m[1].toLowerCase();
        if (key === 'title') {
          headerTitle = m[2];
        } else if (key === 'toc') {
          headerToc = m[2];
        }
      }
      title = headerToc || headerTitle || title;
    }

    const escapedTitle = Handlebars.escapeExpression(title);
    const isCurrent = sanitizedFilename === currentBasename;
    const suffix = isCurrent ? ` ⬅ ${Handlebars.escapeExpression(hereText)}` : '';

    return `  <li><a href="${href}">${escapedTitle}</a>${suffix}</li>`;
  });

  return new Handlebars.SafeString(`<ol>\n${lis.join('\n')}\n</ol>`);
});
