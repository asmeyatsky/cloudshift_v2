import Header from './components/Header'
import TransformView from './components/TransformView'
import SettingsModal from './components/SettingsModal'

export default function App() {
  return (
    <div className="h-screen flex flex-col bg-[#09090b] text-zinc-100 overflow-hidden">
      <Header />
      <TransformView />
      <SettingsModal />
    </div>
  )
}
