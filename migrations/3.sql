CREATE TABLE area (
    id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    type TEXT NOT NULL,
    min_lon REAL NOT NULL,
    min_lat REAL NOT NULL,
    max_lon REAL NOT NULL,
    max_lat REAL NOT NULL
);

INSERT INTO area VALUES ('au', 'Australia', 'country', 112.928889, -55.05, 167.983333, -9.133333);
INSERT INTO area VALUES ('ca', 'Canada', 'country', -141.666667, 40.0, -52.666667, 83.116667);
INSERT INTO area VALUES ('ge', 'Georgia', 'country', 40.013056, 41.15, 46.635556, 043.570556);
INSERT INTO area VALUES ('de', 'Germany', 'country', 5.9, 47.266667, 15.033333, 55.05);
INSERT INTO area VALUES ('uk', 'United Kingdom', 'country', -13.65, 49.866667, 2.866667, 61.5);
INSERT INTO area VALUES ('us', 'United States', 'country', -155.844437, 27.994402, -71.382439, 66.160507);
INSERT INTO area VALUES ('mx', 'Mexico', 'country', -119.921667, 14.55, -86.716667, 32.983333);
INSERT INTO area VALUES ('co', 'Colombia', 'country', -81.85, -4.214722, -66.854722, 13.383333);
INSERT INTO area VALUES ('ar', 'Argentina', 'country', -73.533333, -58.116667, -53.65, -21.783333);
INSERT INTO area VALUES ('th', 'Thailand', 'country', 97.366667, 5.616667, 105.766667, 20.442778);
INSERT INTO area VALUES ('tw', 'Taiwan', 'country', 118.115255566105, 21.733333, 122.107778, 26.389444);
INSERT INTO area VALUES ('nl', 'Netherlands', 'country', 3.133333, 50.75, 7.2, 53.583333);
INSERT INTO area VALUES ('it', 'Italy', 'country', 1.35, 35.483333, 20.433333, 48.533333);
INSERT INTO area VALUES ('es', 'Spain', 'country', -18.166667, 27.633333, 4.333333, 43.916667);
INSERT INTO area VALUES ('cz', 'Czech Republic', 'country', 12.116667, 40.65, 25.5, 59.65);

INSERT INTO area VALUES ('bitcoin-island-philippines', 'Bitcoin Island', 'island', 121.84043884277345, 11.932356183978753, 122.00523376464845, 12.005489474835585);
INSERT INTO area VALUES ('ekasi', 'Bitcoin Ekasi', 'area', 22.10, -34.20, 22.15, -34.15);