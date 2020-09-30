import * as React from "react";
import * as ReactDOM from "react-dom";

export class App extends React.Component {
    constructor(props) {
        super(props);
        this.state = {
            ws: null
        };
    }
    
    get_socket(): null|WebSocket {
        return this.state.ws as null|WebSocket;
    };
    
    componentDidMount() {
        this.connect();
    }
    
    timeout = 250;
    
    build_uri() {
        // We can't assume the server has a certificate.
        let secure   = false;
        
        // If we don't have a hostname, use localhost and pray for the best.
        let hostname = window.location.hostname || 'localhost';
        let port     = window.location.port; // default port
        
        // If client seems to be running over https, use secure connection.
        if((window.location.protocol||'http:') === 'https:') {
            secure = true;
        }
        
        // This is for local testing without server-recompilation.
        if((window.location.protocol||'file:') === 'file:' || hostname === 'localhost') {
            hostname = 'localhost';
            port = '6991';
        }
        
        const HOST = hostname + (port ? ':' + port : '');
        const URI = (secure ? 'wss://' : 'ws://') + HOST + '/websocket';
        
        return {
            hostname: hostname,
            port: port,
            secure: secure,
            host: HOST,
            uri: URI
        };
    }
    
    connect = () => {
        var si = this.build_uri();
        var ws = new WebSocket(si.uri);
        console.log("Connecting to server: ", si);
        
        let that = this;
        var connectInterval;
        
        ws.onopen = () => {
            console.log("Connected to Server", si);
            this.setState({ws: ws});
            that.timeout = 250;
            clearTimeout(connectInterval);
        };
        
        ws.onclose = (ev: CloseEvent) => {
            let next_time = Math.min(1000, that.timeout + that.timeout) / 1000;
            console.log(`WebSocket closed. Reconnecting in ${next_time} seconds...`, ev.reason);
            
            that.timeout = that.timeout + that.timeout;
            connectInterval = setTimeout(this.check, Math.min(1000, that.timeout));
            this.setState({ws: null});
        };
        
        ws.onerror = (err: any) => {
            console.error(`WebSocket encountered an error: `, err, "Closing WebSocket.");
            ws.close();
        };
        
        ws.onmessage = (ev: MessageEvent) => {
            let message_data = ev.data;
            
            if(typeof message_data === "string") {
                let json = JSON.parse(message_data);
                let type = json['type'] ?? null;
                
                if(type === null) {
                    console.error("Message has no type: ", json);
                    return;
                }
                
                console.log("RECV-TXT: ", type, json);
                
                if(typeof json['view'] === "string") {
                    let view_id = json['view'];
                    // TODO: View's?
                }
                
            } else {
                console.log("RECV-BIN: ", typeof message_data);
            }
        };
    }
    
    check = () => {
        const ws = this.get_socket();
        if(!ws || ws.readyState == WebSocket.CLOSED) {
            this.connect();
        }
    };
    
    componentWillUnmount() {
        this.get_socket() && this.get_socket().close();
    }
    
    render() {
        var ws = this.get_socket();
        
        if(ws === null) {
            return <div className="error">No connection.</div>;
        }
        
        return (<div>{ws.readyState}</div>)
    }
}
