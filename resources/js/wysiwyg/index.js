/**
 * WYSIWYG BBCode Editor
 *
 * Main editor class that provides rich text editing for BBCode content.
 * Supports switching between Rich (WYSIWYG) and Raw (BBCode) modes.
 */

import { EditorState } from 'prosemirror-state';
import { EditorView } from 'prosemirror-view';
import { keymap } from 'prosemirror-keymap';
import { history, undo, redo } from 'prosemirror-history';
import { baseKeymap, toggleMark, setBlockType } from 'prosemirror-commands';
import { inputRules, wrappingInputRule, textblockTypeInputRule } from 'prosemirror-inputrules';

import { bbcodeSchema } from './schema.js';
import { parseBBCode, parseHTML } from './bbcode-to-prosemirror.js';
import { serializeToBBCode } from './prosemirror-to-bbcode.js';

/**
 * WYSIWYG Editor class
 */
export class WysiwygEditor {
  /**
   * @param {HTMLTextAreaElement} textarea - Original textarea to enhance
   * @param {Object} options - Configuration options
   */
  constructor(textarea, options = {}) {
    this.textarea = textarea;
    this.options = {
      mode: 'raw', // Start in raw mode, switch to rich after first toggle
      onModeChange: null,
      onContentChange: null,
      ...options
    };

    this.mode = this.options.mode;
    this.editorView = null;
    this.editorContainer = null;
    this.isInitialized = false;

    // Debounce timer for syncing
    this.syncTimer = null;

    this.init();
  }

  /**
   * Initialize the editor
   */
  async init() {
    // Create the editor container
    this.createEditorContainer();

    // Create ProseMirror editor
    this.createEditor();

    // Set initial mode
    if (this.mode === 'rich') {
      await this.switchToRichMode();
    } else {
      this.showRawMode();
    }

    this.isInitialized = true;
  }

  /**
   * Create the container element for the ProseMirror editor
   */
  createEditorContainer() {
    this.editorContainer = document.createElement('div');
    this.editorContainer.className = 'wysiwyg-editor-container';
    this.editorContainer.style.display = 'none';

    // Insert after textarea
    this.textarea.parentNode.insertBefore(
      this.editorContainer,
      this.textarea.nextSibling
    );
  }

  /**
   * Create the ProseMirror editor instance
   */
  createEditor() {
    // Build initial state with empty document
    const state = EditorState.create({
      schema: bbcodeSchema,
      plugins: this.createPlugins()
    });

    // Create editor view
    this.editorView = new EditorView(this.editorContainer, {
      state,
      dispatchTransaction: (transaction) => {
        const newState = this.editorView.state.apply(transaction);
        this.editorView.updateState(newState);

        // Sync to textarea on changes
        if (transaction.docChanged) {
          this.debouncedSync();
          if (this.options.onContentChange) {
            this.options.onContentChange(this.getContent());
          }
        }
      },
      // Handle paste events
      handlePaste: (view, event, slice) => {
        return this.handlePaste(view, event, slice);
      }
    });
  }

  /**
   * Create ProseMirror plugins
   */
  createPlugins() {
    return [
      history(),
      keymap(this.createKeymap()),
      keymap(baseKeymap),
      inputRules({ rules: this.createInputRules() })
    ];
  }

  /**
   * Create keyboard shortcuts
   */
  createKeymap() {
    const schema = bbcodeSchema;
    return {
      'Mod-z': undo,
      'Mod-y': redo,
      'Mod-Shift-z': redo,
      'Mod-b': toggleMark(schema.marks.bold),
      'Mod-i': toggleMark(schema.marks.italic),
      'Mod-u': toggleMark(schema.marks.underline),
      'Mod-Shift-s': toggleMark(schema.marks.strikethrough),
      'Mod-Shift-x': toggleMark(schema.marks.code),
    };
  }

  /**
   * Create input rules for automatic formatting
   */
  createInputRules() {
    const schema = bbcodeSchema;
    return [
      // Bullet list: "- " at start of line
      wrappingInputRule(
        /^\s*[-*]\s$/,
        schema.nodes.bullet_list
      ),
      // Ordered list: "1. " at start of line
      wrappingInputRule(
        /^\s*(\d+)\.\s$/,
        schema.nodes.ordered_list,
        match => ({ order: +match[1] }),
        (match, node) => node.childCount + node.attrs.order === +match[1]
      ),
      // Code block: "```" at start of line
      textblockTypeInputRule(
        /^```(\w+)?\s$/,
        schema.nodes.code_block,
        match => ({ language: match[1] || null })
      ),
      // Horizontal rule: "---" or "***"
      // (handled separately since it creates a node, not modifies a block)
    ];
  }

  /**
   * Handle paste events
   */
  handlePaste(view, event, slice) {
    const clipboardData = event.clipboardData;
    if (!clipboardData) return false;

    // Check for BBCode patterns in plain text
    const text = clipboardData.getData('text/plain');
    if (text && this.containsBBCode(text)) {
      event.preventDefault();
      this.insertBBCode(text);
      return true;
    }

    // Let default handling process HTML
    return false;
  }

  /**
   * Check if text contains BBCode patterns
   */
  containsBBCode(text) {
    // Common BBCode patterns
    const patterns = [
      /\[b\]/i,
      /\[i\]/i,
      /\[u\]/i,
      /\[url/i,
      /\[img/i,
      /\[quote/i,
      /\[code/i,
      /\[spoiler/i,
      /\[color/i,
      /\[size/i
    ];
    return patterns.some(p => p.test(text));
  }

  /**
   * Insert BBCode text by parsing it first
   */
  async insertBBCode(bbcode) {
    try {
      const doc = await parseBBCode(bbcode);
      const tr = this.editorView.state.tr;
      const { from } = tr.selection;
      tr.replaceWith(from, from, doc.content);
      this.editorView.dispatch(tr);
    } catch (error) {
      console.error('Failed to insert BBCode:', error);
      // Fallback: insert as plain text
      const tr = this.editorView.state.tr;
      tr.insertText(bbcode);
      this.editorView.dispatch(tr);
    }
  }

  /**
   * Debounced sync to textarea
   */
  debouncedSync() {
    if (this.syncTimer) {
      clearTimeout(this.syncTimer);
    }
    this.syncTimer = setTimeout(() => {
      this.syncToTextarea();
    }, 300);
  }

  /**
   * Sync ProseMirror content to textarea
   */
  syncToTextarea() {
    if (this.mode === 'rich' && this.editorView) {
      const bbcode = serializeToBBCode(this.editorView.state.doc);
      this.textarea.value = bbcode;

      // Trigger input event for character counter and draft save
      this.textarea.dispatchEvent(new Event('input', { bubbles: true }));
    }
  }

  /**
   * Get current content as BBCode
   */
  getContent() {
    if (this.mode === 'rich' && this.editorView) {
      return serializeToBBCode(this.editorView.state.doc);
    }
    return this.textarea.value;
  }

  /**
   * Set content from BBCode
   */
  async setContent(bbcode) {
    this.textarea.value = bbcode;

    if (this.mode === 'rich' && this.editorView) {
      const doc = await parseBBCode(bbcode);
      const state = EditorState.create({
        doc,
        schema: bbcodeSchema,
        plugins: this.createPlugins()
      });
      this.editorView.updateState(state);
    }
  }

  /**
   * Switch to rich (WYSIWYG) mode
   */
  async switchToRichMode() {
    if (this.mode === 'rich') return;

    // Parse current BBCode content
    const bbcode = this.textarea.value;
    const doc = await parseBBCode(bbcode);

    // Update editor state with parsed document
    const state = EditorState.create({
      doc,
      schema: bbcodeSchema,
      plugins: this.createPlugins()
    });
    this.editorView.updateState(state);

    // Show editor, hide textarea
    this.textarea.style.display = 'none';
    this.editorContainer.style.display = 'block';

    this.mode = 'rich';

    if (this.options.onModeChange) {
      this.options.onModeChange('rich');
    }

    // Focus editor
    this.editorView.focus();
  }

  /**
   * Switch to raw (BBCode) mode
   */
  switchToRawMode() {
    if (this.mode === 'raw') return;

    // Sync content to textarea first
    this.syncToTextarea();

    // Show textarea, hide editor
    this.textarea.style.display = '';
    this.editorContainer.style.display = 'none';

    this.mode = 'raw';

    if (this.options.onModeChange) {
      this.options.onModeChange('raw');
    }

    // Focus textarea
    this.textarea.focus();
  }

  /**
   * Show raw mode (initial setup without sync)
   */
  showRawMode() {
    this.textarea.style.display = '';
    this.editorContainer.style.display = 'none';
    this.mode = 'raw';
  }

  /**
   * Toggle between rich and raw modes
   */
  async toggleMode() {
    if (this.mode === 'raw') {
      await this.switchToRichMode();
    } else {
      this.switchToRawMode();
    }
  }

  /**
   * Check if currently in rich mode
   */
  isRichMode() {
    return this.mode === 'rich';
  }

  /**
   * Focus the editor
   */
  focus() {
    if (this.mode === 'rich' && this.editorView) {
      this.editorView.focus();
    } else {
      this.textarea.focus();
    }
  }

  /**
   * Execute a formatting command
   */
  executeCommand(command, attrs = {}) {
    if (this.mode !== 'rich' || !this.editorView) return false;

    const state = this.editorView.state;
    const dispatch = this.editorView.dispatch.bind(this.editorView);
    const schema = bbcodeSchema;

    switch (command) {
      case 'bold':
        return toggleMark(schema.marks.bold)(state, dispatch);
      case 'italic':
        return toggleMark(schema.marks.italic)(state, dispatch);
      case 'underline':
        return toggleMark(schema.marks.underline)(state, dispatch);
      case 'strikethrough':
        return toggleMark(schema.marks.strikethrough)(state, dispatch);
      case 'color':
        return this.applyMark('color', { color: attrs.color });
      case 'size':
        return this.applyMark('size', { size: attrs.size });
      case 'link':
        return this.applyMark('link', { href: attrs.href });
      case 'insertImage':
        return this.insertNode('image', attrs);
      case 'insertQuote':
        return this.insertNode('quote', attrs);
      case 'insertSpoiler':
        return this.insertNode('spoiler', attrs);
      case 'insertCode':
        return this.insertNode('code_block', attrs);
      default:
        return false;
    }
  }

  /**
   * Apply a mark to the current selection
   */
  applyMark(markName, attrs) {
    const state = this.editorView.state;
    const { from, to } = state.selection;
    const mark = bbcodeSchema.marks[markName].create(attrs);

    const tr = state.tr.addMark(from, to, mark);
    this.editorView.dispatch(tr);
    return true;
  }

  /**
   * Insert a node at the current position
   */
  insertNode(nodeType, attrs) {
    const state = this.editorView.state;
    const { from } = state.selection;
    const schema = bbcodeSchema;

    let node;
    switch (nodeType) {
      case 'image':
        node = schema.nodes.image.create(attrs);
        break;
      case 'quote':
        node = schema.nodes.quote.create(attrs, schema.nodes.paragraph.create());
        break;
      case 'spoiler':
        node = schema.nodes.spoiler.create(attrs, schema.nodes.paragraph.create());
        break;
      case 'code_block':
        node = schema.nodes.code_block.create(attrs);
        break;
      default:
        return false;
    }

    const tr = state.tr.insert(from, node);
    this.editorView.dispatch(tr);
    return true;
  }

  /**
   * Insert text at cursor position
   */
  insertText(text) {
    if (this.mode === 'rich' && this.editorView) {
      const state = this.editorView.state;
      const tr = state.tr.insertText(text);
      this.editorView.dispatch(tr);
    } else {
      // Insert into textarea
      const start = this.textarea.selectionStart;
      const end = this.textarea.selectionEnd;
      const before = this.textarea.value.substring(0, start);
      const after = this.textarea.value.substring(end);
      this.textarea.value = before + text + after;
      this.textarea.selectionStart = this.textarea.selectionEnd = start + text.length;
    }
  }

  /**
   * Insert BBCode-formatted content
   */
  async insertBBCodeContent(bbcode) {
    if (this.mode === 'rich') {
      await this.insertBBCode(bbcode);
    } else {
      this.insertText(bbcode);
    }
  }

  /**
   * Check if a mark is active in the current selection
   */
  isMarkActive(markName) {
    if (this.mode !== 'rich' || !this.editorView) return false;

    const state = this.editorView.state;
    const { from, $from, to, empty } = state.selection;
    const markType = bbcodeSchema.marks[markName];

    if (!markType) return false;

    if (empty) {
      return !!markType.isInSet(state.storedMarks || $from.marks());
    } else {
      return state.doc.rangeHasMark(from, to, markType);
    }
  }

  /**
   * Destroy the editor and clean up
   */
  destroy() {
    if (this.syncTimer) {
      clearTimeout(this.syncTimer);
    }

    if (this.editorView) {
      this.editorView.destroy();
    }

    if (this.editorContainer && this.editorContainer.parentNode) {
      this.editorContainer.parentNode.removeChild(this.editorContainer);
    }

    this.textarea.style.display = '';
  }
}

// Export for use in toolbar
export { bbcodeSchema, parseBBCode, serializeToBBCode };
