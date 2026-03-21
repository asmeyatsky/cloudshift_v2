import { Component, type ReactNode } from 'react'

type Props = { children: ReactNode }
type State = { hasError: boolean; error: Error | null }

export default class ErrorBoundary extends Component<Props, State> {
  state: State = { hasError: false, error: null }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('ErrorBoundary caught:', error, info.componentStack)
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="h-screen flex items-center justify-center bg-[#09090b] text-zinc-100">
          <div className="max-w-md text-center space-y-4">
            <h1 className="text-xl font-semibold text-red-400">Something went wrong</h1>
            <p className="text-sm text-zinc-400">
              {this.state.error?.message || 'An unexpected error occurred.'}
            </p>
            <button
              onClick={() => window.location.reload()}
              className="px-4 py-2 bg-blue-600 rounded text-sm hover:bg-blue-500"
            >
              Reload
            </button>
          </div>
        </div>
      )
    }
    return this.props.children
  }
}
