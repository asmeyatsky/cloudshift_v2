import { driver, type DriveStep } from 'driver.js'
import 'driver.js/dist/driver.css'
import './tour.css'

function tourDriver(steps: DriveStep[]) {
  const filtered = steps.filter((s) => {
    if (s.element == null) return true
    const sel = typeof s.element === 'string' ? s.element : null
    if (!sel) return true
    return document.querySelector(sel) != null
  })
  if (filtered.length === 0) return null
  return driver({
    showProgress: true,
    animate: true,
    smoothScroll: true,
    overlayColor: '#09090b',
    overlayOpacity: 0.88,
    stagePadding: 8,
    stageRadius: 8,
    popoverClass: 'cloudshift-driver-popover',
    nextBtnText: 'Next',
    prevBtnText: 'Back',
    doneBtnText: 'Done',
    progressText: '{{current}} of {{total}}',
    allowClose: true,
    steps: filtered,
  })
}

export function runHomeTour() {
  requestAnimationFrame(() => {
    const d = tourDriver(homeSteps)
    d?.drive()
  })
}

export function runWorkspaceTour() {
  requestAnimationFrame(() => {
    const batch = document.querySelector('#tour-ws-batch')
    const steps = [...workspaceStepsBase]
    if (batch) {
      steps.splice(1, 0, {
        element: '#tour-ws-batch',
        popover: {
          title: 'Batch file list',
          description:
            'When you import multiple files or a repo, each file appears here. Transform this file or Transform all (rate-limited).',
          side: 'right',
        },
      })
    }
    const d = tourDriver(steps)
    d?.drive()
  })
}

const homeSteps: DriveStep[] = [
  {
    popover: {
      title: 'Welcome to CloudShift',
      description:
        'CloudShift rewrites AWS and Azure code toward Google Cloud patterns. This tour walks through the home menu—where you bring code in. Click Next to continue.',
      side: 'over',
    },
  },
  {
    element: '#tour-header',
    popover: {
      title: 'Header',
      description:
        'Branding and version. API ready (or a sign-in hint) shows whether transforms work. The gear opens Settings—API key, history, and guided tours. From the editor, Menu returns here.',
      side: 'bottom',
    },
  },
  {
    element: '#tour-home-intro',
    popover: {
      title: 'What you can do',
      description:
        'From here you open code via GitHub, file/folder/ZIP upload, or paste. Everything opens in the workspace editor for transform.',
      side: 'bottom',
    },
  },
  {
    element: '#tour-home-github',
    popover: {
      title: 'GitHub repository',
      description:
        'Paste a repo URL (https://github.com/org/repo). Pick source cloud (AWS/Azure/auto), optional branch or tag. The server downloads the archive and extracts supported files—same limits as ZIP upload.',
      side: 'bottom',
    },
  },
  {
    element: '#tour-home-imports',
    popover: {
      title: 'Quick imports',
      description:
        'Empty editor opens a blank workspace. Upload file(s): one file in the editor, many as a batch. Folder or ZIP for a whole tree. You can also drag and drop files or a .zip onto the page.',
      side: 'top',
    },
  },
  {
    element: '#tour-home-paste',
    popover: {
      title: 'Paste snippet',
      description:
        'Paste a snippet, choose language and source cloud (AWS / Azure / auto), then Open in editor.',
      side: 'top',
    },
  },
  {
    element: '#tour-home-examples',
    popover: {
      title: 'Service examples',
      description:
        'Dropdown of AWS and Azure SDK samples—source code you migrate from, not GCP. Load one and run Transform in the workspace to see GCP-oriented output.',
      side: 'top',
    },
  },
  {
    popover: {
      title: 'You’re set',
      description:
        'Open any option above, then use Help (?) in the header for the Editor tour. Re-run tours anytime from Settings.',
      side: 'over',
    },
  },
]

const workspaceStepsBase: DriveStep[] = [
  {
    popover: {
      title: 'Workspace',
      description:
        'Left: source code. Right: result after transform. Very large AWS files (many boto3 services in one file) often need splitting—see the amber warning if it appears. Menu returns home.',
      side: 'over',
    },
  },
  {
    element: '#tour-ws-source-toolbar',
    popover: {
      title: 'Source toolbar',
      description:
        'Language and source cloud (AWS / Azure / auto). Path hint helps the engine. Transform (or This file in batch) runs the pipeline. Shortcut: Ctrl+Enter or ⌘+Enter.',
      side: 'bottom',
    },
  },
  {
    element: '#tour-ws-editor',
    popover: {
      title: 'Source editor',
      description:
        'Edit your AWS or Azure code here. In batch mode, the sidebar switches the active file.',
      side: 'right',
    },
  },
  {
    element: '#tour-ws-result',
    popover: {
      title: 'Result panel',
      description:
        'After transform: Diff (before/after), Code (output only), Patterns and warnings. Copy saves the transformed source.',
      side: 'left',
    },
  },
  {
    element: '#tour-insights-bar',
    popover: {
      title: 'Insights bar',
      description:
        'Summary: pattern count, warnings, confidence. Click to expand details. Shown after a successful transform.',
      side: 'top',
    },
  },
  {
    popover: {
      title: 'Tips',
      description:
        'If the insights bar is hidden, run Transform first. Settings stores your API key and history. Happy migrating!',
      side: 'over',
    },
  },
]
