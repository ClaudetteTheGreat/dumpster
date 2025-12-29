/**
 * ProseMirror to BBCode Serializer
 *
 * Converts a ProseMirror document back to BBCode text.
 * This is used when:
 * - Switching from Rich mode to Raw mode
 * - Submitting the form (BBCode is the storage format)
 */

import { bbcodeSchema } from './schema.js';

/**
 * Serialize a ProseMirror document to BBCode
 * @param {Node} doc - ProseMirror document node
 * @returns {string} - BBCode text
 */
export function serializeToBBCode(doc) {
  const serializer = new BBCodeSerializer();
  return serializer.serialize(doc);
}

/**
 * BBCode Serializer class
 */
class BBCodeSerializer {
  constructor() {
    this.output = '';
  }

  /**
   * Serialize a document node
   * @param {Node} doc
   * @returns {string}
   */
  serialize(doc) {
    this.output = '';
    this.serializeFragment(doc.content);
    return this.output.trim();
  }

  /**
   * Serialize a fragment (collection of nodes)
   * @param {Fragment} fragment
   */
  serializeFragment(fragment) {
    fragment.forEach((node, offset, index) => {
      this.serializeNode(node, index === fragment.childCount - 1);
    });
  }

  /**
   * Serialize a single node
   * @param {Node} node
   * @param {boolean} isLast - Whether this is the last node in its parent
   */
  serializeNode(node, isLast = false) {
    const handler = this.nodeHandlers[node.type.name];
    if (handler) {
      handler.call(this, node, isLast);
    } else {
      // Unknown node type - try to serialize content
      if (node.isText) {
        this.output += this.serializeTextWithMarks(node);
      } else if (node.content) {
        this.serializeFragment(node.content);
      }
    }
  }

  /**
   * Serialize text node with its marks
   * @param {Node} node
   * @returns {string}
   */
  serializeTextWithMarks(node) {
    let text = node.text || '';

    // Apply marks in order
    node.marks.forEach(mark => {
      const [open, close] = this.getMarkTags(mark);
      text = open + text + close;
    });

    return text;
  }

  /**
   * Get opening and closing BBCode tags for a mark
   * @param {Mark} mark
   * @returns {[string, string]}
   */
  getMarkTags(mark) {
    switch (mark.type.name) {
      case 'bold':
        return ['[b]', '[/b]'];
      case 'italic':
        return ['[i]', '[/i]'];
      case 'underline':
        return ['[u]', '[/u]'];
      case 'strikethrough':
        return ['[s]', '[/s]'];
      case 'color':
        return [`[color=${mark.attrs.color}]`, '[/color]'];
      case 'size':
        return [`[size=${mark.attrs.size}]`, '[/size]'];
      case 'font':
        return [`[font=${mark.attrs.font}]`, '[/font]'];
      case 'link':
        if (mark.attrs.unfurl) {
          return [`[url unfurl]${mark.attrs.href}[/url]`, ''];
        }
        return [`[url=${mark.attrs.href}]`, '[/url]'];
      case 'mention':
        return [`@${mark.attrs.username}`, ''];
      case 'code':
        return ['[code]', '[/code]'];
      default:
        return ['', ''];
    }
  }

  /**
   * Node serialization handlers
   */
  nodeHandlers = {
    // Paragraph
    paragraph: function(node, isLast) {
      this.serializeInlineContent(node);
      if (!isLast) {
        this.output += '\n';
      }
    },

    // Hard break
    hard_break: function() {
      this.output += '\n';
    },

    // Horizontal rule
    horizontal_rule: function() {
      this.output += '[hr]\n';
    },

    // Text
    text: function(node) {
      this.output += this.serializeTextWithMarks(node);
    },

    // Quote
    quote: function(node) {
      const { author, threadId, postId } = node.attrs;
      if (author && threadId && postId) {
        this.output += `[quote=${author};${threadId};${postId}]\n`;
      } else if (author) {
        this.output += `[quote=${author}]\n`;
      } else {
        this.output += '[quote]\n';
      }
      this.serializeFragment(node.content);
      this.output += '\n[/quote]\n';
    },

    // Spoiler
    spoiler: function(node) {
      const title = node.attrs.title;
      if (title && title !== 'Spoiler') {
        this.output += `[spoiler=${title}]\n`;
      } else {
        this.output += '[spoiler]\n';
      }
      this.serializeFragment(node.content);
      this.output += '\n[/spoiler]\n';
    },

    // Code block
    code_block: function(node) {
      const lang = node.attrs.language;
      if (lang) {
        this.output += `[code=${lang}]`;
      } else {
        this.output += '[code]';
      }
      // Code blocks have plain text content
      this.output += node.textContent;
      this.output += '[/code]\n';
    },

    // Image
    image: function(node) {
      const { src, width, height } = node.attrs;
      if (width && height) {
        this.output += `[img=${width}x${height}]${src}[/img]`;
      } else if (width) {
        this.output += `[img=${width}]${src}[/img]`;
      } else {
        this.output += `[img]${src}[/img]`;
      }
    },

    // Video
    video: function(node) {
      this.output += `[video]${node.attrs.src}[/video]\n`;
    },

    // Audio
    audio: function(node) {
      this.output += `[audio]${node.attrs.src}[/audio]\n`;
    },

    // YouTube
    youtube: function(node) {
      this.output += `[youtube]${node.attrs.videoId}[/youtube]\n`;
    },

    // Bullet list
    bullet_list: function(node) {
      this.output += '[list]\n';
      node.content.forEach(item => {
        this.output += '[*]';
        this.serializeListItemContent(item);
        this.output += '\n';
      });
      this.output += '[/list]\n';
    },

    // Ordered list
    ordered_list: function(node) {
      const listType = node.attrs.listType || '1';
      this.output += `[list=${listType}]\n`;
      node.content.forEach(item => {
        this.output += '[*]';
        this.serializeListItemContent(item);
        this.output += '\n';
      });
      this.output += '[/list]\n';
    },

    // List item (handled by parent list nodes)
    list_item: function(node) {
      this.serializeListItemContent(node);
    },

    // Table
    table: function(node) {
      this.output += '[table]\n';
      this.serializeFragment(node.content);
      this.output += '[/table]\n';
    },

    // Table row
    table_row: function(node) {
      this.output += '[tr]';
      this.serializeFragment(node.content);
      this.output += '[/tr]\n';
    },

    // Table cell
    table_cell: function(node) {
      this.output += '[td]';
      this.serializeInlineContent(node.content.firstChild);
      this.output += '[/td]';
    },

    // Table header
    table_header: function(node) {
      this.output += '[th]';
      this.serializeInlineContent(node.content.firstChild);
      this.output += '[/th]';
    },

    // Center alignment
    center: function(node) {
      this.output += '[center]';
      this.serializeFragment(node.content);
      this.output += '[/center]\n';
    },

    // Left alignment
    align_left: function(node) {
      this.output += '[left]';
      this.serializeFragment(node.content);
      this.output += '[/left]\n';
    },

    // Right alignment
    align_right: function(node) {
      this.output += '[right]';
      this.serializeFragment(node.content);
      this.output += '[/right]\n';
    }
  };

  /**
   * Serialize inline content (text and inline nodes)
   * @param {Node} node
   */
  serializeInlineContent(node) {
    if (!node || !node.content) return;

    node.content.forEach(child => {
      if (child.isText) {
        this.output += this.serializeTextWithMarks(child);
      } else {
        this.serializeNode(child);
      }
    });
  }

  /**
   * Serialize list item content (inline only, no block structure)
   * @param {Node} item
   */
  serializeListItemContent(item) {
    // List items contain paragraphs, we just want their inline content
    if (item.content && item.content.firstChild) {
      this.serializeInlineContent(item.content.firstChild);
    }
  }
}

export { BBCodeSerializer };
