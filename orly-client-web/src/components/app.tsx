import * as React from "react";
import {useState, useEffect} from "react";
import * as ReactDOM from "react-dom";
import { connect, useSelector } from 'react-redux';
import { Connection } from "../connection";
import { AppState } from "../state";
import "./app.scss";

export function AppRoot() {
    const webstate = useSelector<AppState>(state => state._webstate);
    
    if(webstate === 'offline') {
        return <div className='app-superstate app-offline'>Offline.</div>;
    }
    
    if(webstate === 'connecting') {
        return <div className='app-superstate app-connecting'>Connecting...</div>;
    }
    
    if(webstate !== 'online') {
        return <div className='app-superstate app-error'>ERROR</div>;
    }
    
    let main = <div className='app-content-container'></div>;
    
    if( 2*2 > 2 ) {
        main = <AppChannelView />
    }
    
    return <>
        <AppNavigationDrawer />
        {main}
    </>
};

const AppNavigationDrawer = () => {
    const client = useSelector<AppState, object>(state => state.client) as object;
    
    let client_info = client ? <div className='client-info'>
        Client ID: {client?.id}
    </div> : <div>Client ID: Unknown</div>;
    
    return <nav className='app-navigation-drawer'>
        <div style={{flex: '1 0 auto'}}></div>
        {client_info}
    </nav>
};

const AppChannelView = () => {
    const messages = useSelector<AppState, object>(state => state.messages) as Array<object>;
    
    return <main className='app-content-container app-channel-view'>
        <div style={{flex: '1 0 auto'}}>
            {messages.map(message => {
                return <div dangerouslySetInnerHTML={{__html: message.message}}></div>;
            })}
        </div>
        <MessageComposer />
    </main>
};

/*
class MessageComposer extends React.Component {
    websocket: null|WebSocket = null;
    
    constructor(props) {
        super(props);
        this.state = {message: ""};
        this.websocket = this.context.store._webhook;
        
        ReactReduxContext.Consumer.
    }
    
    change = (event) => {
        this.setState({message: event.target.value});
    };
    
    submit = (event: React.FormEvent<HTMLFormElement>) => {
        
        let store = this.context.store._websocket;
        debugger;
        
        this.setState({message: ""});
        event.preventDefault();
        return false;
    };
    
    render() {
        return <form className='message-composer' onSubmit={(event) => {return this.submit(event)}}>
            <input type='text'
                value={this.state.message}
                onChange={this.change}
                onInput={this.change}
            />
            <button type='submit'>SEND</button>
        </form>
    }
}
//*/

let MessageComposer = ({dispatch}) => {
    let [message, setMessage] = useState("");
    
    let submit = (event) => {
        dispatch((dispatch, getState, {connection}: {connection: Connection}) => {
            connection.send_txt("channel.broadcast.formatted", {
                message: message
            });
        });
        
        setMessage("");
        event.preventDefault();
        return false;
    };
    
    return <form className='message-composer' onSubmit={(event) => submit(event)}>
        <input type='text'
                value={message}
                onChange={(event) => {setMessage(event.target.value)}}
                onInput={(event) => {setMessage(event.target.value)}}
            />
            <button type='submit'>SEND</button>
    </form>
};
MessageComposer = connect()(MessageComposer);
