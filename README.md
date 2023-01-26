# Fishinge
The other half of my learning project, utilizing my [eventsub_websocket](https://github.com/Fittiboy/eventsub_websocket) crate to listen to a specific Twitch channel point reward redemption.  
When it is redeemd, it updates a StreamElements chat bot command.  

The process (lots of details omitted):  
1. Create WebSocket connection to Twitch's EventSub server (and handle the entire protocol in the background)
1. Create EventSub subscription to a channel point reward redemption for our WebSocket connection, through Twitch's Helix API
1. When event occurs, update a bot command through the StreamElements API, wait five minutes, and then reset the command back to its original state
  
![Peek 2023-01-25 15-53](https://user-images.githubusercontent.com/28876473/214760201-4c57ba92-1c5e-4fd2-bc66-00c5ec09c4aa.gif)
