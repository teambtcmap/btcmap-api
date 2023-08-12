#!/bin/bash

sqlite3 ~/.local/share/btcmap/btcmap.db "update element set osm_json = json('{}'), tags = json('{}') where id = (select id from element where osm_json != json('{}') and deleted_at != '' limit 1);"

count=$(sqlite3 ~/.local/share/btcmap/btcmap.db "select count (*) from element where deleted_at != '' and osm_json != json('{}');")
echo "Remaining: $count"
