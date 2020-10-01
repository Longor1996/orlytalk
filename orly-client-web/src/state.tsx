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
    clients: {};
    messages: [];
}

export declare type AppStateAction = any;

const initial_app_state: AppState = {
    _serverinfo: null,
    _websocket: null,
    _webstate: 'offline',
    client: null,
    clients: {},
    messages: [],
};

const app_state_producer = produce((draft: Draft<AppState>, action: AppStateAction) => {
    // do nothing for now
    console.log("app_state_reduce", draft, action);
    let action_type = action.type;
    
    if(action_type === WS.ACTION_WEBSOCKET_INIT) {
        draft._webstate = 'connecting';
    }
    
    if(action_type === WS.ACTION_WEBSOCKET_OPEN) {
        draft._webstate = 'online';
    }
    
    if(action_type === WS.ACTION_WEBSOCKET_RECVTXT+'client-info.self') {
        let client = Object.freeze(action.client);
        draft.client = client;
        draft.clients[client.id] = client;
    }
    
    if(action_type === WS.ACTION_WEBSOCKET_RECVTXT+'channel.broadcast.formatted') {
        let message = Object.freeze({
            message: action.message,
            client: action.client,
        });
        
        draft.messages.push(message);
    }
    
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
