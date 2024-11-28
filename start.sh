cp /home/serv/sweat_voter/sweat_voter/participants.json /home/serv/sweat_voter/backup/participants.json
cd client
trunk build
cd ..
cp /home/serv/sweat_voter/backup/participants.json /home/serv/sweat_voter/sweat_voter/participants.json 
cargo run --package server