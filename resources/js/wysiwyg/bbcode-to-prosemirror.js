/**
 * BBCode to ProseMirror Parser
 *
 * Converts BBCode text to a ProseMirror document by:
 * 1. Using the server's /api/bbcode/preview endpoint to get rendered HTML
 * 2. Parsing the HTML using ProseMirror's DOMParser
 *
 * This ensures the WYSIWYG view matches exactly what users see in posts.
 */

import { DOMParser } from 'prosemirror-model';
import { bbcodeSchema } from './schema.js';

/**
 * Parse BBCode string into a ProseMirror document
 * @param {string} bbcode - Raw BBCode text
 * @returns {Promise<Node>} - ProseMirror document node
 */
export async function parseBBCode(bbcode) {
  if (!bbcode || bbcode.trim() === '') {
    // Return empty document with single paragraph
    return bbcodeSchema.node('doc', null, [
      bbcodeSchema.node('paragraph')
    ]);
  }

  try {
    // Fetch rendered HTML from server
    const html = await fetchRenderedHTML(bbcode);

    // Parse HTML into ProseMirror document
    return parseHTML(html);
  } catch (error) {
    console.error('Failed to parse BBCode:', error);
    // Fallback: create document with raw text
    return createFallbackDocument(bbcode);
  }
}

/**
 * Fetch rendered HTML from the BBCode preview API
 * @param {string} bbcode - Raw BBCode text
 * @returns {Promise<string>} - Rendered HTML
 */
async function fetchRenderedHTML(bbcode) {
  const response = await fetch('/api/bbcode/preview', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({ content: bbcode })
  });

  if (!response.ok) {
    throw new Error(`BBCode preview failed: ${response.status}`);
  }

  const data = await response.json();
  return data.html || '';
}

/**
 * Parse HTML string into a ProseMirror document
 * @param {string} html - Rendered HTML
 * @returns {Node} - ProseMirror document node
 */
export function parseHTML(html) {
  // Create a temporary DOM element to parse the HTML
  const container = document.createElement('div');
  container.innerHTML = html;

  // Pre-process the DOM to handle special elements
  preprocessDOM(container);

  // Use ProseMirror's DOMParser with our schema
  const parser = DOMParser.fromSchema(bbcodeSchema);
  return parser.parse(container);
}

/**
 * Pre-process DOM to handle elements that need special treatment
 * before ProseMirror parsing
 * @param {HTMLElement} container
 */
function preprocessDOM(container) {
  // Handle blockquotes with attribution
  container.querySelectorAll('blockquote').forEach(bq => {
    // Extract author from "username said:" pattern
    const header = bq.querySelector('.quote-header, .bbcode-quote-header');
    if (header) {
      const authorText = header.textContent || '';
      const authorMatch = authorText.match(/^(.+?)\s+said:/i);
      if (authorMatch) {
        bq.setAttribute('data-author', authorMatch[1].trim());
      }

      // Extract thread/post IDs from link if present
      const link = header.querySelector('a');
      if (link) {
        const href = link.getAttribute('href') || '';
        const match = href.match(/\/threads\/(\d+)\/post-(\d+)/);
        if (match) {
          bq.setAttribute('data-thread-id', match[1]);
          bq.setAttribute('data-post-id', match[2]);
        } else {
          const postMatch = href.match(/#post-(\d+)/);
          if (postMatch) {
            bq.setAttribute('data-post-id', postMatch[1]);
          }
        }
      }
    }
  });

  // Handle spoilers
  container.querySelectorAll('.bbcode-spoiler, details').forEach(spoiler => {
    spoiler.classList.add('bbcode-spoiler');
    const summary = spoiler.querySelector('summary');
    if (summary && summary.textContent) {
      // Title is already accessible via summary element
    }
  });

  // Handle code blocks with language
  container.querySelectorAll('pre code').forEach(code => {
    const langClass = Array.from(code.classList).find(c => c.startsWith('language-'));
    if (langClass) {
      code.parentElement.setAttribute('data-language', langClass.replace('language-', ''));
    }
  });

  // Handle YouTube embeds
  container.querySelectorAll('iframe').forEach(iframe => {
    const src = iframe.getAttribute('src') || '';
    if (src.includes('youtube')) {
      iframe.classList.add('youtube-embed');
    }
  });

  // Handle mentions
  container.querySelectorAll('a.mention').forEach(mention => {
    const href = mention.getAttribute('href') || '';
    const usernameMatch = href.match(/\/members\/([^/]+)/);
    if (usernameMatch) {
      mention.setAttribute('data-username', decodeURIComponent(usernameMatch[1]));
    }
  });

  // Handle text alignment
  container.querySelectorAll('[style*="text-align"]').forEach(el => {
    const style = el.getAttribute('style') || '';
    if (style.includes('text-align: center')) {
      el.classList.add('text-center');
    } else if (style.includes('text-align: right')) {
      el.classList.add('text-right');
    } else if (style.includes('text-align: left')) {
      el.classList.add('text-left');
    }
  });
}

/**
 * Create a fallback document when parsing fails
 * @param {string} text - Raw text to include
 * @returns {Node} - ProseMirror document node
 */
function createFallbackDocument(text) {
  const lines = text.split('\n');
  const content = lines.map(line => {
    if (line.trim() === '') {
      return bbcodeSchema.node('paragraph');
    }
    return bbcodeSchema.node('paragraph', null, [
      bbcodeSchema.text(line)
    ]);
  });

  return bbcodeSchema.node('doc', null, content);
}

/**
 * Synchronous HTML parsing (for when HTML is already available)
 * @param {string} html - Rendered HTML string
 * @returns {Node} - ProseMirror document node
 */
export function parseHTMLSync(html) {
  return parseHTML(html);
}
