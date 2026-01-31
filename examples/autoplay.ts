// Autoplay
import { EventLoop, WebViewBuilder,
    WindowBuilder 
} from "../index.js";
const eventLoop = new EventLoop();
const window = new WindowBuilder().build(eventLoop)

const webview = new WebViewBuilder()
    .withHtml(`<!DOCTYPE html>
    <html>
        <head>
            <title>Webview</title>
        </head>
        <body>
            <h1 id="output">Hello world!</h1>
            <video autoplay>
                <source src="https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4" type="video/mp4">
            </video>
        </body>
    </html>
    `)
    .build(eventLoop,'0')
console.log(webview.id)
//if (!webview.isDevtoolsOpen()) webview.openDevtools();
eventLoop.run()
// Now run the app with a polling loop to allow IPC callbacks to process
const poll = () => {
    if (eventLoop.runIteration()) {
        window.id;
        webview.id;
        setTimeout(poll, 10);
        } else {
            process.exit(0);
            }
            };
            setInterval(() => {
                console.log("polling");
                }, 1000);
                poll();
                //app.run();
                /*
                */