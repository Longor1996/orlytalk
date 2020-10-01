import * as React from 'react';
import * as ReactDOM from "react-dom";
import { Provider } from 'react-redux';

import {app_state_create} from "./state";
import {AppRoot} from "./components/app";
import {initiate_connection} from './connection';

var connection = initiate_connection();
var app_root = document.getElementById('app-root');
var app_state_store = app_state_create({
    connection: connection
});

ReactDOM.render(
    <Provider store={app_state_store}>
        <AppRoot/>
    </Provider>,
    app_root
);

window.requestAnimationFrame(() => {
    connection.connect(app_state_store);
});
