import { createStore, applyMiddleware, compose, Store, Reducer } from 'redux';
import thunk from 'redux-thunk';
import { enableMapSet } from 'immer';
enableMapSet();
import { produce, Draft } from 'immer';
import * as WS from './connection';

export interface AppState {
    _serverinfo: null|object;
    _websocket: null|WebSocket;
    _webstate: string;
    client: null|object;
    clients: {[key: number]: object};
    messages: Array<any>;
    ui: AppUiState;
}

export interface AppUiState {
    navdrawer: boolean,
    usrdrawer: boolean,
}

export declare type AppStateAction = any;

const initial_app_state: AppState = {
    _serverinfo: null,
    _websocket: null,
    _webstate: 'offline',
    client: null,
    clients: {},
    messages: [],
    ui: {
        navdrawer: false,
        usrdrawer: false
    },
};

const app_state_producer = produce((draft: Draft<AppState>, action: AppStateAction) => {
    // do nothing for now
    let action_type = action.type;
    
    if(action_type === WS.ACTION_WEBSOCKET_INIT) {
        draft._webstate = 'connecting';
        return;
    }
    
    if(action_type === WS.ACTION_WEBSOCKET_OPEN) {
        draft._webstate = 'online';
        return;
    }
    
    if(action_type === WS.ACTION_WEBSOCKET_RECVTXT+'client-info.self') {
        let client = Object.freeze(action.client);
        draft.client = client;
        draft.clients[client.id] = client;
        return;
    }
    
    if(action_type === WS.ACTION_WEBSOCKET_RECVTXT+'channel.broadcast.formatted') {
        let message = Object.freeze({
            message: action.message,
            client: action.client,
        });
        
        draft.messages.push(message);
        return;
    }
    
    if(action_type === 'ui:nav-drawer-toggle') {
        draft.ui.navdrawer = !(!!draft.ui.navdrawer);
        return;
    }
    
    console.log("app_state_reduce", draft, action);
}, initial_app_state);

export declare type AppStateReducer = Reducer<AppState, AppStateAction>;
export function app_state_reducer(state: AppState = initial_app_state, action: AppStateAction) {
    return app_state_producer(state, action);
}

export declare type AppStateStore = Store<AppState, AppStateAction>;
export function app_state_create(thunkExtra) {
    let composeEnhancers = compose;
    let middlewares = [
        thunk.withExtraArgument(thunkExtra)
    ];
    
    if(window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__) {
        composeEnhancers = window.__REDUX_DEVTOOLS_EXTENSION_COMPOSE__({
            //trace: true
        });
    }
    
    const enhancer = composeEnhancers(
        applyMiddleware(...middlewares)
    );
    
    return createStore(
        app_state_reducer,
        initial_app_state,
        enhancer
    );
}
