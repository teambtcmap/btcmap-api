# BTC Map Areas RPC

Methods for managing geographic areas on BTC Map. Areas are GeoJSON polygons (typically countries or communities) that group together the map elements within their boundary. Each area has its own row in the `area` table and is identified by a numeric `id` or a string `url_alias` tag.

- [add_area](add_area.md) - Create a new area
- [generate_area_bboxes](generate_area_bboxes.md) - Recompute the bounding box of every area from its GeoJSON
- [generate_areas_elements_mapping](generate_areas_elements_mapping.md) - Recompute which elements belong to which areas
- [generate_reports](generate_reports.md) - Generate daily area reports
- [get_area](get_area.md) - Retrieve a single area by id or alias
- [get_area_dashboard](get_area_dashboard.md) - 365-day element and verification charts for an area
- [get_most_commented_countries](get_most_commented_countries.md) - Countries with the most comments in a given period
- [get_trending_communities](get_trending_communities.md) - Communities trending in a given period
- [get_trending_countries](get_trending_countries.md) - Countries trending in a given period
- [remove_area](remove_area.md) - Soft-delete an area
- [remove_area_tag](remove_area_tag.md) - Remove a tag from an area
- [set_area_image](set_area_image.md) - Set the square or wide image for an area
- [set_area_tag](set_area_tag.md) - Set or update a tag on an area
