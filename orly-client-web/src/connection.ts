import {AppState, AppStateStore} from './state';

export const WEBSOCKET_DEFAULT_PORT = '6991';
export const WEBSOCKET_DEFAULT_TIMEOUT = 750;

function build_uri() {
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
        port = WEBSOCKET_DEFAULT_PORT;
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

export const ACTION_WEBSOCKET_INIT = 'websocket:init';
export const ACTION_WEBSOCKET_OPEN = 'websocket:open';
export const ACTION_WEBSOCKET_ERROR = 'websocket:error';
export const ACTION_WEBSOCKET_CLOSE = 'websocket:close';

export const ACTION_WEBSOCKET_RECVTXT = 'websocket:recv-txt:';
export const ACTION_WEBSOCKET_RECVBIN = 'websocket:recv-bin:';

export function initiate_connection() {
    var si = null;
    var ws: null|WebSocket = null;
    var store = null;
    var timeout = WEBSOCKET_DEFAULT_TIMEOUT;
    
    var connect = (store_in: AppStateStore) => {
        store = store_in;
        si = build_uri();
        ws = new WebSocket(si.uri);
        
        store.dispatch({
            type: ACTION_WEBSOCKET_INIT,
            _server_info: si,
            _websocket: ws,
        });
        
        var connectInterval;
        
        ws.onopen = () => {
            store.dispatch({
                type: ACTION_WEBSOCKET_OPEN,
                _server_info: si,
                _websocket: ws,
            });
            
            timeout = WEBSOCKET_DEFAULT_TIMEOUT;
            clearTimeout(connectInterval);
        };
            
        ws.onclose = (ev: CloseEvent) => {
            if(ws === null) {
                return;
            }
            
            let next_time = Math.min(1000, timeout + timeout) / 1000;
            console.log(`WebSocket closed, reconnecting in ${next_time} seconds...`, ev.reason);
            
            timeout = timeout + timeout;
            connectInterval = setTimeout(check, Math.min(1000, timeout));
            
            store.dispatch({
                type: ACTION_WEBSOCKET_CLOSE,
                reason: ev.reason,
                timeout: timeout
            });
        };
        
        ws.onerror = (err: any) => {
            console.error(`WebSocket encountered an error and is closing: `, err);
            
            store.dispatch({
                type: ACTION_WEBSOCKET_ERROR,
                _server_info: si,
                _websocket: ws,
                error: err,
                timeout: timeout
            });
            
            ws && ws.close();
        };
        
        ws.onmessage = (ev: MessageEvent) => {
            let message_data = ev.data;
            
            if(typeof message_data === "string") {
                let json = JSON.parse(message_data);
                json._server_info = si;
                json._websocket = ws;
                
                let type = json['type'] ?? null;
                if(type === null) {
                    console.error("Message has no type: ", json);
                    return;
                }
                
                json.type = ACTION_WEBSOCKET_RECVTXT + json.type;
                store.dispatch(json);
            } else {
                store.dispatch({
                    type: ACTION_WEBSOCKET_RECVBIN + '?',
                    _server_info: si,
                    _websocket: ws,
                    payload: message_data
                });
            }
        };
        
    };
    
    var check = () => {
        if(store && (!ws || ws.readyState == WebSocket.CLOSED)) {
            connect(store);
        }
    };
    
    var send_raw = (data) => {
        if(ws !== null) {
            ws.send(data);
            return true;
        } else {
            return false;
        }
    };
    
    var send_txt = (type: string, json: any) => send_raw(JSON.stringify({
        ...json, type: type
    }));
    
    window.addEventListener('beforeunload', () => {
        ws && ws.close();
        ws = null;
    });
    
    window.addEventListener('unload', () => {
        ws && ws.close();
        ws = null;
    });
    
    return {
        connect: connect,
        send_raw: send_raw,
        send_txt: send_txt,
    };
}

export type Connection = ReturnType<typeof initiate_connection>;
