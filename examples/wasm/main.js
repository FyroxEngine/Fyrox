const moduleGame = import('./pkg/wasm.js').then(({ default: init, main_js }) =>
  init().then(() => main_js)
)
const elementTargetButton = document.querySelector('#button-start')
const elementmain = document.querySelector('#main')

const run = async () => {
  elementTargetButton.removeEventListener('click', run)
  elementmain.remove()

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
