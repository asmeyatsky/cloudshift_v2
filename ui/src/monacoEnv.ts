/**
 * Load Monaco from the bundled npm package (not jsDelivr) so CSP script-src 'self' works.
 */
import { loader } from '@monaco-editor/react'
import * as monaco from 'monaco-editor'
import editorWorker from 'monaco-editor/esm/vs/editor/editor.worker?worker'
import cssWorker from 'monaco-editor/esm/vs/language/css/css.worker?worker'
import htmlWorker from 'monaco-editor/esm/vs/language/html/html.worker?worker'
import jsonWorker from 'monaco-editor/esm/vs/language/json/json.worker?worker'
import tsWorker from 'monaco-editor/esm/vs/language/typescript/ts.worker?worker'

declare let self: Window &
  typeof globalThis & {
    MonacoEnvironment?: { getWorker(moduleId: string, label: string): Worker }
  }

self.MonacoEnvironment = {
  getWorker(_moduleId: string, label: string) {
    switch (label) {
      case 'json':
        return new jsonWorker()
      case 'css':
      case 'scss':
      case 'less':
        return new cssWorker()
      case 'html':
      case 'handlebars':
      case 'razor':
        return new htmlWorker()
      case 'typescript':
      case 'javascript':
      case 'tsx':
      case 'jsx':
        return new tsWorker()
      default:
        return new editorWorker()
    }
  },
}

loader.config({ monaco })
