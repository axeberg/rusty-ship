const WS_URL = `ws://localhost:3000/ws`;

const ws = new WebSocket(`${WS_URL}`);

ws.addEventListener('open', () => {
  console.log('[CLIENT]: Hi there server!');
});

ws.addEventListener('message', (event) => {
  console.log(`[SERVER]: ${event.data}`);
});

setTimeout(() => {
  ws.send('[ALL DONE]');
  console.log(`[CLIENT]: Closing the WebSocket`);
  ws.close(3000, `Time to die`);
}, 10000);