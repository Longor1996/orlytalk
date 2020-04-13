const domContainer = document.body as HTMLBodyElement;
var CC: null|ClientConnection = null;

class LoginScreen {
    element: HTMLElement;
    
    constructor() {
        this.element = document.createElement('form');
        this.element.style.display = 'flex';
        this.element.style.flexFlow = 'column nowrap';
        this.element.style.position = 'absolute';
        this.element.style.top = '50%';
        this.element.style.left = '50%';
        this.element.style.transform = 'translate(-50%, -50%)';
        this.element.style.fontSize = '1.5rem';
        
        this.element.style.background = '#333';
        this.element.style.borderRadius = '3px';
        this.element.style.border = '1px solid rgba(255,255,255,0.25)';
        this.element.style.padding = '0.5rem';
        
        /*
        var text = document.createElement('h3');
        text.innerText = "Login...";
        text.style.margin = '0.25rem 0';
        this.element.appendChild(text);
        //*/
        
        var input = document.createElement('input');
        input.placeholder = 'Username...';
        input.style.marginBottom = '0.5rem';
        input.style.padding = '0.25rem';
        this.element.appendChild(input);
        
        input.addEventListener('keyup', (evt) => {
            if(evt.key !== 'Enter') return;
            button.click();
        });
        
        var button = document.createElement('button');
        button.style.alignSelf = 'flex-end';
        button.style.padding = '0.25rem';
        button.innerText = 'LOGIN';
        this.element.appendChild(button);
        
        button.addEventListener('click', (evt) => {
            evt.preventDefault();
            this.tryLogin(input.value);
        });
        
        domContainer.appendChild(this.element);
        
        setTimeout(() => input.focus(), 500);
    }
    
    tryLogin(name: string) {
        this.setLockState(true);
        
        const HOST = location.port !== '6991' ? "localhost:6991" : location.host;
        let uri = 'ws://' + HOST + '/websocket?name=' + name;
        let ws = new WebSocket(uri);
        
        ws.onopen = (event) => {
            this.destroy();
            CC = new ClientConnection(ws, name);
        };
        
        ws.onerror = (event) => {
            this.setLockState(false);
            console.error(event);
        };
        
        ws.onclose = (event) => {
            this.setLockState(false);
        };
        
        ws.onmessage = (event) => {
            // nothing to do here
        };
    }
    
    setLockState(locked: boolean) {
        if(locked) {
            this.element.style.pointerEvents = 'none';
            this.element.style.userSelect = 'none';
            this.element.style.opacity = '0.5';
        } else {
            this.element.style.pointerEvents = '';
            this.element.style.userSelect = '';
            this.element.style.opacity = '';
        }
    }
    
    destroy() {
        this.element.remove();
        domContainer.innerHTML = "";
    }
}

class ClientConnection {
    ws: WebSocket;
    clients: ClientInfoCache;
    users_panel: ClientsPanel;
    
    screens: any  = {};
    screen: null|ContentScreen = null;
    
    constructor(ws: WebSocket, name: string) {
        this.ws = ws;
        this.clients = new ClientInfoCache(this);
        
        this.ws.onerror = (event) => {
            console.error(event);
        };
        
        this.ws.onclose = (event) => {
            this.destroy();
        };
        
        this.ws.onmessage = (event) => {
            // nothing to do here
            let text = event.data;
            let json = JSON.parse(text);
            let type = json['type'] ?? null;
            
            if(type === null) {
                console.error("Message has no type: ", json);
                return;
            }
            
            console.log("RECV", type, json);
            
            if(typeof json['screen_id'] !== "undefined") {
                let screen_id = json['screen_id'];
                let screen: null|ContentScreen = this.screens[screen_id] ?? null;
                if(screen !== null) {
                    screen.recv(type, json);
                } else {
                    console.error("Unknown Screen: ", screen_id);
                }
            } else {
                this.recv(type, json);
            }
        };
        
        this.users_panel = new ClientsPanel(this);
        
        let default_screen = new ChannelScreen(this, 'default');
        this.screens[default_screen.id] = default_screen;
        this.setActiveScreen(default_screen.id);
    }
    
    setActiveScreen(screen_id: string) {
        if(this.screen !== null) {
            this.screen.deactivate();
        }
        
        this.screen = this.screens[screen_id] ?? null;
        if(this.screen === null) {
            return;
        }
        
        this.screen.activate();
    }
    
    recv(type: string, json: any) {
        
        if(type === "user-info.self") {
            let ci = ClientInfo.from_json(json.user);
            this.clients.put(ci);
        }
        
        if(type === "user-info.list") {
            let users = json.users as Array<any>;
            for(let i=0,item=null;i<users.length,item=users[i];i++) {
                this.clients.put(ClientInfo.from_json(item));
            }
        }
        
        if(type === "user.join") {
            this.clients.put(ClientInfo.from_json(json.user));
        }
        
        if(type === "user.leave") {
            this.clients.del(json.user);
        }
        
    }
    
    send(type: string, screen_id: null|string, json: any) {
        json.type = type;
        if(screen_id !== null) {
            json.screen_id = screen_id;
        }
        
        console.log("SEND", type, screen_id, json);
        this.ws.send(JSON.stringify(json));
    }
    
    destroy() {
        CC = null;
        domContainer.innerHTML = "";
        new LoginScreen();
    }
}

class ClientInfo {
    uuid: string;
    name: string;
    
    constructor(uuid: string, name: string) {
        this.uuid = uuid;
        this.name = name;
    }
    
    static from_json(json: any) {
        return new this(json.uuid, json.name);
    }
    
}

class ClientInfoCache {
    cc: ClientConnection;
    cache: any;
    
    constructor(cc: ClientConnection) {
        this.cc = cc;
        this.cache = {};
    }
    
    put(client: ClientInfo) {
        if(typeof this.cache[client.uuid] !== "undefined") {
            throw new Error("User already exists: "+client.uuid);
        }
        
        this.cache[client.uuid] = client;
        this.cc.users_panel.update(client.uuid, 'insert');
    }
    
    get(uuid: string): null|ClientInfo {
        return this.cache[uuid] ?? null;
    }
    
    del(uuid: string) {
        this.cc.users_panel.update(uuid, 'delete');
        delete this.cache[uuid];
    }
}

class ClientsPanel {
    cc: ClientConnection;
    element: HTMLElement;
    
    constructor(cc: ClientConnection) {
        this.cc = cc;
        
        this.element = document.createElement('div');
        this.element.className = "users-panel";
        this.element.style.display = "flex";
        this.element.style.flexFlow = "column nowrap";
        this.element.style.background = "#333";
        domContainer.appendChild(this.element);
    }
    
    update(uuid: string, mode: string) {
        
        if(mode === "insert") {
            console.log("Adding new user to panel: ", uuid);
            let box = document.createElement('div');
            box.className = "users-panel_user";
            box.style.display = "inline-block";
            box.style.margin = "0.25rem";
            box.setAttribute('data-uuid', uuid);
            box.innerHTML = this.cc.clients.get(uuid)?.name ?? "Guest";
            this.element.appendChild(box);
            return;
        }
        
        if(mode === "delete") {
            console.log("Removing user from panel: ", uuid);
            this.element.querySelector(`.users-panel_user[data-uuid="${uuid}"]`)?.remove();
        }
        
    }
}

abstract class ContentScreen {
    static counter: number = 0;
    cc: ClientConnection;
    id: string;
    element: HTMLElement;
    
    constructor(cc: ClientConnection, id: string) {
        this.cc = cc;
        this.id = id;
        this.element = document.createElement('div');
        this.element.id = 'screen-'+this.id;
        this.element.className = "screen";
        this.element.style.display = 'none';
        domContainer.appendChild(this.element);
    }
    
    abstract recv(type: string, json: any): void;
    
    send(type: string, json: any) {
        this.cc.send(type, this.id, json);
    }
    
    activate() {
        this.element.style.display = '';
    };
    deactivate() {
        this.element.style.display = 'none';
    };
}

class ChannelScreen extends ContentScreen {
    constructor(cc: ClientConnection, id: string) {
        super(cc, id);
        
        let styles = `
            #${this.element.id} {
                flex: 1 1 auto;
                overflow: hidden auto;
                
                display: flex;
                flex-flow: column nowrap;
                margin-bottom: 3rem;
            }
            
            #${this.element.id}::after {
                content: '';
                display: block;
                min-height: 0.5rem;
            }
            
            #${this.element.id} .message-input {
                position: absolute;
                left:0;right:0;bottom:0;
                height: 3rem;
                
                display: flex;
                flex-flow: row nowrap;
                
                background: #333;
                border-top: 1px solid grey;
            }
            
            #${this.element.id} .message-input input {
                flex: 1 1 auto;
                font-size: 1.25rem;
                padding: 0.25rem;
                
                border: none;
                background: #383838;
                color: white;
            }
            
            #${this.element.id} .message {
                display: block;
            }
            
            #${this.element.id} .message .message-time {
                font-family: 'system-mono';
                font-size: 0.75rem;
                margin-right: 0.25rem;
                color: grey;
            }
            
            #${this.element.id} .message .message-user {
                color: cornflowerblue;
                font-weight: bold;
            }
            
            #${this.element.id} .message .message-text {
                display: inline-block;
            }
        `;
        
        let style = document.createElement('style');
        style.id = 'screen-'+this.id+'_style';
        style.innerHTML = styles;
        document.head.appendChild(style);
        
        let input = document.createElement('div');
        input.className = "message-input";
        
        let input_txt = document.createElement('input');
        let input_btn = document.createElement('button');
        
        input_txt.placeholder = "Message #" + this.id;
        input_btn.innerHTML = "SEND";
        
        input_txt.addEventListener('keyup', (evt) => {
            if(evt.key !== 'Enter') return;
            input_btn.click();
        });
        
        input_btn.addEventListener('click', () => {
            let text = input_txt.value.trim();
            input_txt.value = "";
            if(text.length === 0) {
                return;
            }
            
            this.send('user.message', {
                message: text
            });
        });
        
        input.appendChild(input_txt);
        input.appendChild(input_btn);
        this.element.appendChild(input);
        
        setTimeout(() => input_txt.focus(), 500);
    }
    
    recv(type: string, json: any) {
        if(type === "user.message") {
            this.show_message(json.user, json.message);
        }
    }
    
    show_message(user_id: null|string, message: string) {
        let dt = new Date();
        
        let box = document.createElement('div');
        box.className = "message";
        
        let time_h = ""+dt.getHours();
        let time_m = ""+dt.getMinutes();
        if(time_h.length===1) time_h = "0" + time_h;
        if(time_m.length===1) time_m = "0" + time_m;
        
        let time_span = document.createElement('span');
        time_span.className = "message-time";
        time_span.innerHTML = time_h + ':' + time_m;
        box.appendChild(time_span);
        
        if(user_id !== null) {
            let user = this.cc.clients.get(user_id);
            let user_span = document.createElement('span');
            user_span.className = "message-user";
            user_span.innerHTML = user?.name ?? 'Guest';
            box.appendChild(user_span);
            
            let user_sepr = document.createElement('span');
            user_sepr.className = "message-user-separator";
            user_sepr.innerHTML = ":&nbsp;";
            box.appendChild(user_sepr);
        }
        
        let text_elmt = document.createElement('div');
        text_elmt.className = "message-text";
        text_elmt.innerHTML = message;
        box.appendChild(text_elmt);
        
        this.element.appendChild(box);
        this.element.scrollTop = box.offsetTop;
    }
    
    activate(): void {
        super.activate();
    }
    
    deactivate(): void {
        super.deactivate();
    }
}

new LoginScreen();
