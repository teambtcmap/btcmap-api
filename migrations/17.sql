ALTER TABLE area ADD COLUMN tags TEXT NOT NULL DEFAULT '{}';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/S8SjacANWN') where id = 'praia-bitcoin-jeri';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/3djSVkgrcB') where id = 'bitcoin-ekasi';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/n9e2vmeD5s') where id = 'rome-bitcoin-forum';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/YAZjxHysQD') where id = 'bitcoin-island-philippines';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/wY63zKVXBX') where id = 'bitcoin-lake-guatemala';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/sghZy3gQCV') where id = 'bitcoin-lausanne';
UPDATE area SET tags = json_set(tags, '$.contact:telegram', 'https://t.me/BitcoinLausanne') where id = 'bitcoin-lausanne';
UPDATE area SET tags = json_set(tags, '$.contact:twitter', 'https://twitter.com/BitcoinLausanne') where id = 'bitcoin-lausanne';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/GeVCY6H2KY') where id = 'bitcoin-naples';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/GR6xs4zVBJ') where id = 'bitcoin-rock';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/22vY9BGcd9') where id = 'free-madeira';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/AmYMBwHh5Z') where id = 'iom';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/Gdqv5ERSJN') where id = 'london-bitcoin-space';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/Hr3dHeUseR') where id = 'lugano-plan-b';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/NW2s6MXFUX') where id = 'round-rock-bitcoiners';

UPDATE area SET tags = json_set(tags, '$.contact:discord', 'https://discord.gg/zDX3rnkhn7') where id = 'tokyo-citadel';
UPDATE area SET tags = json_set(tags, '$.contact:website', 'http://tokyocitadel.com/') where id = 'tokyo-citadel';
UPDATE area SET tags = json_set(tags, '$.contact:twitter', 'https://twitter.com/tokyocitadel') where id = 'tokyo-citadel';

CREATE TRIGGER area_updated_at UPDATE OF tags, deleted_at ON area
BEGIN
    UPDATE area SET updated_at = strftime('%Y-%m-%dT%H:%M:%SZ') WHERE id = old.id;
END;