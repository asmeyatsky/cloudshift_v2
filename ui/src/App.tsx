import { useEffect } from 'react'
import Header from './components/Header'
import HomeView from './components/HomeView'
import TransformView from './components/TransformView'
import SettingsModal from './components/SettingsModal'
import { checkAuth } from './api'
import { useStore } from './store'

export default function App() {
  const setAuthVerified = useStore((s) => s.setAuthVerified)
  const apiKey = useStore((s) => s.apiKey)
  const screen = useStore((s) => s.screen)

  useEffect(() => {
    let cancelled = false
    ;(async () => {
      const ok = await checkAuth(apiKey || undefined)
      if (!cancelled) setAuthVerified(ok)
    })()
    return () => {
      cancelled = true
    }
  }, [apiKey, setAuthVerified])

  return (
    <div className="h-screen flex flex-col bg-[#09090b] text-zinc-100 overflow-hidden">
      <Header />
      {screen === 'home' ? <HomeView /> : <TransformView />}
      <SettingsModal />
    </div>
  )
}
