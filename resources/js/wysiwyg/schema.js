/**
 * ProseMirror schema for BBCode WYSIWYG editor
 *
 * This schema defines nodes and marks that map to BBCode elements,
 * enabling bidirectional conversion between ProseMirror documents and BBCode.
 */

import { Schema } from 'prosemirror-model';

// =============================================================================
// Node Specifications
// =============================================================================

const nodes = {
  // The root document node
  doc: {
    content: 'block+'
  },

  // Basic paragraph - default block element
  paragraph: {
    content: 'inline*',
    group: 'block',
    parseDOM: [{ tag: 'p' }],
    toDOM() { return ['p', 0]; }
  },

  // Hard line break - [br] or shift+enter
  hard_break: {
    inline: true,
    group: 'inline',
    selectable: false,
    parseDOM: [{ tag: 'br' }],
    toDOM() { return ['br']; }
  },

  // Horizontal rule - [hr]
  horizontal_rule: {
    group: 'block',
    parseDOM: [{ tag: 'hr' }],
    toDOM() { return ['hr']; }
  },

  // Plain text node
  text: {
    group: 'inline'
  },

  // ==========================================================================
  // Quote node - [quote] and [quote=author;threadId;postId]
  // ==========================================================================
  quote: {
    content: 'block+',
    group: 'block',
    attrs: {
      author: { default: null },
      threadId: { default: null },
      postId: { default: null }
    },
    parseDOM: [{
      tag: 'blockquote',
      getAttrs(dom) {
        return {
          author: dom.getAttribute('data-author') || null,
          threadId: dom.getAttribute('data-thread-id') || null,
          postId: dom.getAttribute('data-post-id') || null
        };
      }
    }],
    toDOM(node) {
      const attrs = { class: 'bbcode-quote' };
      if (node.attrs.author) attrs['data-author'] = node.attrs.author;
      if (node.attrs.threadId) attrs['data-thread-id'] = node.attrs.threadId;
      if (node.attrs.postId) attrs['data-post-id'] = node.attrs.postId;

      // Build the quote structure with optional attribution
      if (node.attrs.author) {
        return ['blockquote', attrs,
          ['div', { class: 'quote-header' },
            ['span', { class: 'quote-author' }, node.attrs.author + ' said:'],
            node.attrs.postId ? ['a', {
              class: 'quote-link',
              href: node.attrs.threadId
                ? `/threads/${node.attrs.threadId}/post-${node.attrs.postId}`
                : `#post-${node.attrs.postId}`
            }, '\u2191'] : ''
          ],
          ['div', { class: 'quote-content' }, 0]
        ];
      }
      return ['blockquote', attrs, ['div', { class: 'quote-content' }, 0]];
    }
  },

  // ==========================================================================
  // Spoiler node - [spoiler] and [spoiler=title]
  // ==========================================================================
  spoiler: {
    content: 'block+',
    group: 'block',
    attrs: {
      title: { default: 'Spoiler' }
    },
    parseDOM: [{
      tag: 'details.bbcode-spoiler',
      getAttrs(dom) {
        const summary = dom.querySelector('summary');
        return {
          title: summary ? summary.textContent : 'Spoiler'
        };
      }
    }],
    toDOM(node) {
      return ['details', { class: 'bbcode-spoiler', open: 'open' },
        ['summary', node.attrs.title],
        ['div', { class: 'spoiler-content' }, 0]
      ];
    }
  },

  // ==========================================================================
  // Code block - [code] and [code=language]
  // ==========================================================================
  code_block: {
    content: 'text*',
    group: 'block',
    code: true,
    defining: true,
    marks: '',
    attrs: {
      language: { default: null }
    },
    parseDOM: [{
      tag: 'pre',
      preserveWhitespace: 'full',
      getAttrs(dom) {
        const code = dom.querySelector('code');
        if (code) {
          // Extract language from class like "language-javascript"
          const match = code.className.match(/language-(\w+)/);
          return { language: match ? match[1] : null };
        }
        return { language: null };
      }
    }],
    toDOM(node) {
      const codeAttrs = {};
      if (node.attrs.language) {
        codeAttrs.class = `language-${node.attrs.language}`;
      }
      return ['pre', { class: 'bbcode-code' }, ['code', codeAttrs, 0]];
    }
  },

  // ==========================================================================
  // Image - [img], [img=WxH], [img=W]
  // ==========================================================================
  image: {
    inline: true,
    group: 'inline',
    draggable: true,
    attrs: {
      src: {},
      width: { default: null },
      height: { default: null },
      alt: { default: '' }
    },
    parseDOM: [{
      tag: 'img[src]:not(.bbcode-thumb-img)',
      getAttrs(dom) {
        return {
          src: dom.getAttribute('src'),
          width: dom.getAttribute('width') || null,
          height: dom.getAttribute('height') || null,
          alt: dom.getAttribute('alt') || ''
        };
      }
    }],
    toDOM(node) {
      const attrs = { src: node.attrs.src, class: 'bbcode-image' };
      if (node.attrs.width) attrs.width = node.attrs.width;
      if (node.attrs.height) attrs.height = node.attrs.height;
      if (node.attrs.alt) attrs.alt = node.attrs.alt;
      return ['img', attrs];
    }
  },

  // ==========================================================================
  // Thumbnail - [thumb] - clickable image that links to full size
  // ==========================================================================
  thumbnail: {
    inline: true,
    group: 'inline',
    atom: true,
    draggable: true,
    attrs: {
      src: {}
    },
    parseDOM: [{
      tag: 'a.bbcode-thumb',
      getAttrs(dom) {
        const img = dom.querySelector('img');
        return {
          src: img ? img.getAttribute('src') : dom.getAttribute('href')
        };
      }
    }],
    toDOM(node) {
      return ['a', { href: node.attrs.src, class: 'bbcode-thumb', target: '_blank' },
        ['img', { src: node.attrs.src, class: 'bbcode-thumb-img' }]
      ];
    }
  },

  // ==========================================================================
  // Video embed - [video]
  // ==========================================================================
  video: {
    group: 'block',
    attrs: {
      src: {}
    },
    parseDOM: [{
      tag: 'video[src]',
      getAttrs(dom) {
        return { src: dom.getAttribute('src') };
      }
    }],
    toDOM(node) {
      return ['div', { class: 'bbcode-video-wrapper' },
        ['video', { src: node.attrs.src, controls: 'controls', class: 'bbcode-video' }]
      ];
    }
  },

  // ==========================================================================
  // Audio embed - [audio]
  // ==========================================================================
  audio: {
    group: 'block',
    attrs: {
      src: {}
    },
    parseDOM: [{
      tag: 'audio[src]',
      getAttrs(dom) {
        return { src: dom.getAttribute('src') };
      }
    }],
    toDOM(node) {
      return ['div', { class: 'bbcode-audio-wrapper' },
        ['audio', { src: node.attrs.src, controls: 'controls', class: 'bbcode-audio' }]
      ];
    }
  },

  // ==========================================================================
  // YouTube embed - [youtube]videoId[/youtube]
  // ==========================================================================
  youtube: {
    group: 'block',
    attrs: {
      videoId: {}
    },
    parseDOM: [{
      tag: 'iframe.youtube-embed',
      getAttrs(dom) {
        const src = dom.getAttribute('src') || '';
        const match = src.match(/youtube(?:-nocookie)?\.com\/embed\/([^?]+)/);
        return { videoId: match ? match[1] : '' };
      }
    }],
    toDOM(node) {
      return ['div', { class: 'bbcode-youtube-wrapper responsive-embed' },
        ['iframe', {
          class: 'youtube-embed',
          src: `https://www.youtube-nocookie.com/embed/${node.attrs.videoId}`,
          frameborder: '0',
          allowfullscreen: 'allowfullscreen'
        }]
      ];
    }
  },

  // ==========================================================================
  // Lists - [list], [list=1], [list=a]
  // ==========================================================================
  bullet_list: {
    content: 'list_item+',
    group: 'block',
    parseDOM: [{ tag: 'ul' }],
    toDOM() { return ['ul', { class: 'bbcode-list' }, 0]; }
  },

  ordered_list: {
    content: 'list_item+',
    group: 'block',
    attrs: {
      listType: { default: '1' } // '1' for numeric, 'a' for alphabetic
    },
    parseDOM: [
      {
        tag: 'ol',
        getAttrs(dom) {
          const type = dom.getAttribute('type');
          return { listType: type === 'a' ? 'a' : '1' };
        }
      }
    ],
    toDOM(node) {
      const attrs = { class: 'bbcode-list' };
      if (node.attrs.listType === 'a') attrs.type = 'a';
      return ['ol', attrs, 0];
    }
  },

  list_item: {
    content: 'paragraph block*',
    parseDOM: [{ tag: 'li' }],
    toDOM() { return ['li', 0]; },
    defining: true
  },

  // ==========================================================================
  // Tables - [table], [tr], [td], [th]
  // ==========================================================================
  table: {
    content: 'table_row+',
    group: 'block',
    tableRole: 'table',
    isolating: true,
    parseDOM: [{ tag: 'table' }],
    toDOM() { return ['table', { class: 'bbcode-table' }, ['tbody', 0]]; }
  },

  table_row: {
    content: '(table_cell | table_header)+',
    tableRole: 'row',
    parseDOM: [{ tag: 'tr' }],
    toDOM() { return ['tr', 0]; }
  },

  table_cell: {
    content: 'block+',
    tableRole: 'cell',
    isolating: true,
    parseDOM: [{ tag: 'td' }],
    toDOM() { return ['td', 0]; }
  },

  table_header: {
    content: 'block+',
    tableRole: 'header_cell',
    isolating: true,
    parseDOM: [{ tag: 'th' }],
    toDOM() { return ['th', 0]; }
  },

  // ==========================================================================
  // Text alignment wrappers - [center], [left], [right]
  // ==========================================================================
  center: {
    content: 'block+',
    group: 'block',
    parseDOM: [{ tag: 'div.text-center' }],
    toDOM() { return ['div', { class: 'text-center', style: 'text-align: center;' }, 0]; }
  },

  align_left: {
    content: 'block+',
    group: 'block',
    parseDOM: [{ tag: 'div.text-left' }],
    toDOM() { return ['div', { class: 'text-left', style: 'text-align: left;' }, 0]; }
  },

  align_right: {
    content: 'block+',
    group: 'block',
    parseDOM: [{ tag: 'div.text-right' }],
    toDOM() { return ['div', { class: 'text-right', style: 'text-align: right;' }, 0]; }
  }
};

// =============================================================================
// Mark Specifications (inline formatting)
// =============================================================================

const marks = {
  // Bold - [b]
  bold: {
    parseDOM: [
      { tag: 'strong' },
      { tag: 'b' },
      { style: 'font-weight', getAttrs: value => /^(bold|700|800|900)$/.test(value) && null }
    ],
    toDOM() { return ['strong', 0]; }
  },

  // Italic - [i]
  italic: {
    parseDOM: [
      { tag: 'em' },
      { tag: 'i' },
      { style: 'font-style=italic' }
    ],
    toDOM() { return ['em', 0]; }
  },

  // Underline - [u]
  underline: {
    parseDOM: [
      { tag: 'u' },
      { style: 'text-decoration', getAttrs: value => value.includes('underline') && null }
    ],
    toDOM() { return ['u', 0]; }
  },

  // Strikethrough - [s]
  strikethrough: {
    parseDOM: [
      { tag: 's' },
      { tag: 'strike' },
      { tag: 'del' },
      { style: 'text-decoration', getAttrs: value => value.includes('line-through') && null }
    ],
    toDOM() { return ['s', 0]; }
  },

  // Color - [color=red] or [color=#ff0000]
  color: {
    attrs: {
      color: {}
    },
    parseDOM: [{
      style: 'color',
      getAttrs: value => value ? { color: value } : false
    }],
    toDOM(mark) {
      return ['span', { style: `color: ${mark.attrs.color};` }, 0];
    }
  },

  // Size - [size=8] to [size=36]
  size: {
    attrs: {
      size: {}
    },
    parseDOM: [{
      style: 'font-size',
      getAttrs: value => value ? { size: parseInt(value) || 14 } : false
    }],
    toDOM(mark) {
      return ['span', { style: `font-size: ${mark.attrs.size}px;` }, 0];
    }
  },

  // Font - [font=arial]
  font: {
    attrs: {
      font: {}
    },
    parseDOM: [{
      style: 'font-family',
      getAttrs: value => value ? { font: value.replace(/['"]/g, '') } : false
    }],
    toDOM(mark) {
      return ['span', { style: `font-family: ${mark.attrs.font};` }, 0];
    }
  },

  // Link - [url] and [url=href]
  link: {
    attrs: {
      href: {},
      unfurl: { default: false }
    },
    inclusive: false,
    parseDOM: [{
      tag: 'a[href]:not(.bbcode-thumb)',
      getAttrs(dom) {
        return {
          href: dom.getAttribute('href'),
          unfurl: dom.classList.contains('unfurl')
        };
      }
    }],
    toDOM(mark) {
      const attrs = { href: mark.attrs.href };
      if (mark.attrs.unfurl) attrs.class = 'unfurl';
      return ['a', attrs, 0];
    }
  },

  // Mention - @username
  mention: {
    attrs: {
      username: {}
    },
    inclusive: false,
    parseDOM: [{
      tag: 'a.mention',
      getAttrs(dom) {
        return { username: dom.getAttribute('data-username') };
      }
    }],
    toDOM(mark) {
      return ['a', {
        class: 'mention',
        href: `/members/${mark.attrs.username}`,
        'data-username': mark.attrs.username
      }, 0];
    }
  },

  // Inline code (for cases where code is used inline)
  code: {
    parseDOM: [{ tag: 'code' }],
    toDOM() { return ['code', { class: 'inline-code' }, 0]; }
  }
};

// =============================================================================
// Create and export the schema
// =============================================================================

export const bbcodeSchema = new Schema({ nodes, marks });

// Export individual specs for potential customization
export { nodes, marks };
