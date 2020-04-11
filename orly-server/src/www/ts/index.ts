import {Converter} from "./showdown.min";

var chat_output = document.querySelector("#chat-output");
var chat_input = document.querySelector("#chat-input") as HTMLElement;
var chat_input_txt = document.querySelector("#chat-input input") as HTMLInputElement;
var chat_input_btn = document.querySelector("#chat-input button") as HTMLButtonElement;

var users = {};
function getUser(uuid) {
    return users[uuid];
}

var my_name = window.prompt("Name?", "Usr" + Math.floor(Math.random() * 9999));

var host = location.port !== '6991' ? "localhost:6991" : location.host;
var uri = 'ws://' + host + '/websocket?name=' + my_name;
var ws = new WebSocket(uri);

var converter = new Converter();

function scroll_to_bottom() {
    window.scrollTo(0, document.body.scrollHeight);
}

function send_message(message) {
    message = message.trim();
    if (message.length === 0) {
        return;
    }
    
    ws.send(JSON.stringify({
        type: 'user.message',
        message: message
    }));
    show_message(my_name, converter.makeHtml(message));
}

function show_message(user, message) {
    var line = document.createElement('div');
    line.classList.add('message');
    line.innerHTML = "<span class=user>" + user + "</span>: <div class=text>" + message + "</div>";
    chat_output.appendChild(line);
    scroll_to_bottom();
}

function show_statemsg(message) {
    var line = document.createElement('p');
    line.classList.add('message', 'status');
    line.innerHTML = message;
    chat_output.appendChild(line);
    scroll_to_bottom();
}

function show_errormsg(message) {
    var line = document.createElement('p');
    line.classList.add('message', 'error');
    line.innerHTML = message;
    chat_output.appendChild(line);
    scroll_to_bottom();
}

ws.onopen = function () {
    chat_output.innerHTML = "<p><em>Connected!</em></p>";
    chat_input_txt.focus();
}
ws.onerror = function () {
    show_errormsg("A WebSocket-error occurred.");
    chat_input.style.display = 'none';
}
ws.onclose = function () {
    show_errormsg("Connection lost.");
    chat_input.style.display = 'none';
}
ws.onmessage = function (msg) {
    var json = null;

    try {
        json = JSON.parse(msg.data);
    } catch (error) {
        console.error(error, msg.data);
        debugger;
    }

    switch (json.type) {
        case "user-list":
            for (var i = 0, item = null; i < json.users.length, item = json.users[i]; i++) {
                users[item.uuid] = item;
            }
            break;
        case "user.join":
            users[json.user.uuid] = json.user;
            show_statemsg(getUser(json.user.uuid).name + " joined the chat.");
            break;
        case "user.message":
            show_message(getUser(json.user).name, json.message);
            break;
        case "user.leave":
            show_statemsg(getUser(json.user).name + " left the chat.");
            break;
        default:
            show_errormsg("Unknown Packet Type: " + json.type + "<br><pre>" + msg.data + "</pre>");
            break;
    }
};

chat_input_txt.addEventListener('keyup', function (evt) {
    if (evt.key !== 'Enter') return;
    var msg = chat_input_txt.value;
    chat_input_txt.value = '';
    send_message(msg);
});

chat_input_btn.onclick = function () {
    var msg = chat_input_txt.value;
    chat_input_txt.value = '';
    send_message(msg);
    chat_input_txt.focus();
};