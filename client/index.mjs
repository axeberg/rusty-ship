import { h, render } from 'https://unpkg.com/preact?module';
import htm from 'https://unpkg.com/htm?module';

const html = htm.bind(h);

function App({ message }) {
  return html` <div>${message}</div> `;
}

const url = new URL('/ws', window.location.href);
url.protocol = url.protocol.replace('http', 'ws');

const ws = new WebSocket(url.href);

ws.onopen = () => {
  console.info('[CLIENT]: Connection opened');
};

ws.onmessage = ({ data }) => {
  render(html`<${App} message=${data}></${App}>`, document.body);
};

ws.onclose = () => {
  console.info('[CLIENT]: Connection closed');
};

setTimeout(() => {
  console.info(`[CLIENT]: Closing the WebSocket`);
  ws.close(3000, `Time to die`);
}, 10000);
