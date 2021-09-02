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
        <AppHeader />
        <AppNavigationDrawer />
        {main}
    </>
};

const AppHeader = connect()(({dispatch}) => {
    return <nav className='app-header'>
        <AppNavigationDrawerToggleButton/ >
        <div style={{flex:'1 1 auto'}}></div>
    </nav>
});

const AppNavigationDrawerToggleButton = connect()(({dispatch}) => {
    const isNavVisible = (useSelector<AppState, boolean>(state => state.ui.navdrawer) as boolean);
    
    let svg = isNavVisible
        ? <svg style={{width:'3rem', height:'3rem'}} viewBox="0 0 24 24"><path fill='white' d="M3,6H21V8H3V6M3,11H21V13H3V11M3,16H21V18H3V16Z" /></svg>
        : <svg style={{width:'3rem', height:'3rem'}} viewBox="0 0 24 24"><path fill='white' d="M21,15.61L19.59,17L14.58,12L19.59,7L21,8.39L17.44,12L21,15.61M3,6H16V8H3V6M3,13V11H13V13H3M3,18V16H16V18H3Z" /></svg>;
    
    return <button
        onClick={()=>{dispatch({type: 'ui:nav-drawer-toggle'})}}
        style={{background:'#444', color: 'white', padding: 0, border: 'none'}}
    >
        {svg}
    </button>;
});

const AppNavigationDrawer = () => {
    const isVisible = (useSelector<AppState, boolean>(state => state.ui.navdrawer) as boolean) ? 'show' : 'hide';
    const client = useSelector<AppState, object>(state => state.client) as object;
    
    let client_info = client ? <div className='client-info'>
        Client ID: {client?.id}
    </div> : <div>Client ID: Unknown</div>;
    
    return <nav className={'app-navigation-drawer ' + isVisible}>
        <div style={{flex: '1 0 auto'}}></div>
        {client_info}
    </nav>
};

const AppChannelView = () => {
    const messages = useSelector<AppState, object>(state => state.messages) as Array<object>;
    
    return <main className='app-content-container app-channel-view'>
        <div className='feed'>
            {messages.map(message => {
                return <div className='post'>
                    <span className='post-user' dangerouslySetInnerHTML={{__html: message.user}}></span>
                    &nbsp;
                    <div className='post-text' dangerouslySetInnerHTML={{__html: message.message}}></div>
                </div>;
            })}
        </div>
        <MessageComposer />
    </main>
};

let MessageComposer = ({dispatch}) => {
    let [message, setMessage] = useState("");
    
    let submit = (event) => {
        dispatch((dispatch, getState, {connection}: {connection: Connection}) => {
            connection.send_txt("channel.broadcast.text.formatted", {
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
