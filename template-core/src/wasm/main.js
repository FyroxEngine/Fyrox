const moduleGame = import('./pkg/executor_wasm.js').then(({ default: init, main }) =>
  init().then(() => main)
)
const elementTargetButton = document.querySelector('#button-start')
const elementMain = document.querySelector('#main')

const run = async () => {
  elementTargetButton.removeEventListener('click', run)
  elementMain.remove()

  const context = new AudioContext()

  if (context.state !== 'running') {
    await context.resume()
  }

  return (await moduleGame)()
}

elementTargetButton.addEventListener('click', run, {
  once: true,
  passive: true,
})
