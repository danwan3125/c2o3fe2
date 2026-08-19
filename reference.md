# How Did I...
Intended as a personal reference to catch up on code logic/implementation/patterns in case I decide to revisit this in the future.
## For the Operator:
+ Implement reconnection?

Have the websockets task spawning for sending and receiving from the Websockets channel in a loop. When the websocket tasks exit for any reason, attempt to reenter the Websockets task spawning sequence by attempting to reconnect to the server, and increase the delay (with jitter) otherwise.
+ Simultaneously send messages from stdin to server, and from server to stdout?
+ Implement the loop to read from stdin?
+ Additional question: why did I use the ws:// scheme to connect to the HTTP to WS upgrade?