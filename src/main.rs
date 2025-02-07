use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde_json::json;
use serde_json::Value;
use csv::{ReaderBuilder, StringRecord};
use std::cmp::min;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;

// Cache file path and duration (12 hours)
const CACHE_FILE: &str = "data_cache.json";
const CACHE_DURATION: Duration = Duration::from_secs(12 * 60 * 60);

/// Fetch the CSV data from Google Sheets and process it into JSON.
async fn fetch_sheet_data_from_google() -> Result<Value, Box<dyn std::error::Error>> {
    // Google Sheet CSV export URL (ensure your sheet is publicly accessible)
    let sheet_url = "https://docs.google.com/spreadsheets/d/18kCz2igidQVgqwLdpsDA15kYXLxqX99r/export?format=csv&gid=1370952005";
    let response = reqwest::get(sheet_url).await?.text().await?;
    
    // Debug: print the first 500 characters of the CSV
    println!(
        "Raw CSV response (first 500 chars): {}",
        &response[..min(response.len(), 500)]
    );
    
    // Remove any potential BOM
    let response = response.trim_start_matches('\u{feff}');
    
    // Auto-detect delimiter: compare counts of commas and semicolons in first line.
    let first_line = response.lines().next().unwrap_or("");
    let comma_count = first_line.matches(',').count();
    let semicolon_count = first_line.matches(';').count();
    let delimiter = if semicolon_count > comma_count { b';' } else { b',' };
    println!("Detected delimiter: '{}'", delimiter as char);
    
    // Build CSV reader WITHOUT auto‑headers.
    let mut rdr = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(response.as_bytes());
    
    let mut header_record: Option<StringRecord> = None;
    let mut records = Vec::new();
    
    // Loop through CSV records
    for result in rdr.records() {
        let record = result?;
        
        // Skip completely empty rows.
        if record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        
        // Look for the header row. In your CSV, the proper header appears when the second field is "Zip".
        if header_record.is_none() {
            if record.len() >= 2 && record.get(1).map(|s| s.trim()) == Some("Zip") {
                header_record = Some(record);
                println!("Found header row: {:?}", header_record);
            }
            continue;
        }
        
        // Process data rows using the found header.
        if let Some(ref header) = header_record {
            let mut json_record = serde_json::Map::new();
            for (i, field) in record.iter().enumerate() {
                let key = match header.get(i) {
                    Some(s) if !s.trim().is_empty() => s.trim().to_string(),
                    _ => format!("column_{}", i),
                };
                json_record.insert(key, json!(field.trim()));
            }
            records.push(Value::Object(json_record));
        }
    }
    
    // Remove unwanted keys ("Country" and "column_0") from each record.
    for rec in records.iter_mut() {
        if let Value::Object(map) = rec {
            map.remove("Country");
            map.remove("column_0");
        }
    }
    
    Ok(json!(records))
}

/// Fetch the sheet data with caching. If the cache file is fresh (under 12 hours old),
/// load the JSON from disk. Otherwise, fetch new data and update the cache.
async fn fetch_sheet_data() -> Result<Value, Box<dyn std::error::Error>> {
    // Check if cache file exists and is fresh.
    if let Ok(metadata) = fs::metadata(CACHE_FILE).await {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed < CACHE_DURATION {
                    println!("Using cached data (age: {:?})", elapsed);
                    let cached_data = fs::read_to_string(CACHE_FILE).await?;
                    let json_data: Value = serde_json::from_str(&cached_data)?;
                    return Ok(json_data);
                }
            }
        }
    }
    
    // Otherwise, fetch fresh data.
    println!("Fetching fresh data from Google Sheets...");
    let json_data = fetch_sheet_data_from_google().await?;
    
    // Save to cache.
    let json_string = serde_json::to_string_pretty(&json_data)?;
    let mut file = fs::File::create(CACHE_FILE).await?;
    file.write_all(json_string.as_bytes()).await?;
    println!("Saved new data to cache.");
    
    Ok(json_data)
}

/// API endpoint: /data returns the CSV-converted JSON.
#[get("/data")]
async fn data_handler() -> impl Responder {
    match fetch_sheet_data().await {
        Ok(json_data) => HttpResponse::Ok().json(json_data),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

/// API endpoint: /supplemental returns supplemental info (links, previews, tags).
#[get("/supplemental")]
async fn supplemental_handler() -> impl Responder {
    match fs::read_to_string("supplemental.json").await {
        Ok(data) => {
            match serde_json::from_str::<Value>(&data) {
                Ok(json_data) => HttpResponse::Ok().json(json_data),
                Err(e) => HttpResponse::InternalServerError().body(format!("Error parsing supplemental JSON: {}", e))
            }
        }
        Err(e) => HttpResponse::InternalServerError().body(format!("Error reading supplemental JSON file: {}", e))
    }
}

/// The root endpoint (/) serves an HTML page with dark mode styling, a search input,
/// a state dropdown, and new card‑style displays. (No results show by default.)
#[get("/")]
async fn index() -> impl Responder {
    let html = r#"
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>GKC Data Dashboard 🌌</title>
  <style>
    /* Dark mode styling */
    body {
      background-color: #121212;
      color: #e0e0e0;
      font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif;
      margin: 0;
      padding: 20px;
    }
    h1 {
      text-align: center;
    }
    /* Search and dropdown container */
    #search-container {
      text-align: center;
      margin-bottom: 20px;
    }
    input[type="text"], select {
      background-color: #1e1e1e;
      border: 1px solid #333;
      color: #e0e0e0;
      padding: 10px;
      border-radius: 5px;
      font-size: 16px;
      margin: 5px;
    }
    input[type="text"]::placeholder {
      color: #777;
    }
    /* Data display cards */
    .card {
      background-color: #1e1e1e;
      border: 1px solid #333;
      border-radius: 8px;
      padding: 15px;
      margin: 10px;
      transition: transform 0.2s;
    }
    .card:hover {
      transform: scale(1.02);
    }
    /* Scrollbar styling for WebKit */
    ::-webkit-scrollbar {
      width: 12px;
    }
    ::-webkit-scrollbar-track {
      background: #1e1e1e;
    }
    ::-webkit-scrollbar-thumb {
      background-color: #333;
      border-radius: 6px;
      border: 3px solid #1e1e1e;
    }
    /* Container for data cards */
    #data-container {
      display: flex;
      flex-wrap: wrap;
      justify-content: center;
    }
    .card p {
      margin: 5px 0;
    }
    /* Emoji styling */
    .emoji {
      margin-right: 5px;
    }
    a {
      color: #00aaff;
      text-decoration: none;
    }
  </style>
</head>
<body>
  <h1>GKC Data Dashboard 🌌</h1>
  <div id="search-container">
    <input id="search-input" type="text" placeholder="🔍 Search by City, County, or Zip" />
    <select id="state-dropdown">
      <option value="">Select State</option>
    </select>
  </div>
  <div id="data-container"></div>
  
  <script>
    let originalData = [];
    let supplementalData = [];

    // Fetch banned area data from the /data endpoint.
    async function fetchData() {
      try {
        const response = await fetch('/data');
        const data = await response.json();
        originalData = data;
        populateStateDropdown(data);
        // Do not display any data initially.
      } catch (error) {
        console.error('Error fetching banned area data:', error);
      }
    }

    // Fetch supplemental info from the /supplemental endpoint.
    async function fetchSupplemental() {
      try {
        const response = await fetch('/supplemental');
        const data = await response.json();
        supplementalData = data;
      } catch (error) {
        console.error('Error fetching supplemental data:', error);
      }
    }
    
    // Build the state dropdown using a fixed list of US states.
    // Only enable states that are present in the data.
    function populateStateDropdown(data) {
      const allStates = ["AL","AK","AZ","AR","CA","CO","CT","DE","FL","GA","HI","ID",
                         "IL","IN","IA","KS","KY","LA","ME","MD","MA","MI","MN","MS",
                         "MO","MT","NE","NV","NH","NJ","NM","NY","NC","ND","OH","OK",
                         "OR","PA","RI","SC","SD","TN","TX","UT","VT","VA","WA","WV",
                         "WI","WY"];
      const availableStates = new Set(data.map(item => item.State));
      const dropdown = document.getElementById('state-dropdown');
      dropdown.innerHTML = '<option value="">Select State</option>';
      allStates.forEach(state => {
        const option = document.createElement('option');
        option.value = state;
        option.textContent = state;
        // Enable the option only if the state is available in the data.
        option.disabled = !availableStates.has(state);
        dropdown.appendChild(option);
      });
    }
    
    // Display the results as cards.
    function displayResults(bannedResults, supplementalResults) {
      const container = document.getElementById('data-container');
      container.innerHTML = '';

      // If no banned area matches and no supplemental info matches,
      // show a “good news” message.
      if(bannedResults.length === 0 && supplementalResults.length === 0){
        container.innerHTML = '<p>No kratom bans found in your area 🎉</p>';
        return;
      }
      
      // If banned area results exist, create a summary card.
      if(bannedResults.length > 0) {
        let matchTypes = new Set();
        bannedResults.forEach(item => {
          const query = document.getElementById('search-input').value.trim().toLowerCase();
          if(item.City && item.City.toLowerCase().includes(query)) matchTypes.add("City");
          if(item.County && item.County.toLowerCase().includes(query)) matchTypes.add("County");
          if(item.Zip && item.Zip.toLowerCase().includes(query)) matchTypes.add("Zip");
          if(item.State && item.State.toLowerCase().includes(query)) matchTypes.add("State");
        });
        const card = document.createElement('div');
        card.className = 'card';
        card.innerHTML = `
          <p><strong>Kratom ban detected!</strong></p>
          <p>Matched by: ${Array.from(matchTypes).join(', ')}</p>
          <p>(${bannedResults.length} area${bannedResults.length > 1 ? 's' : ''} in the data.)</p>
        `;
        container.appendChild(card);
      }
      
      // Display supplemental links (if any).
      supplementalResults.forEach(item => {
        const card = document.createElement('div');
        card.className = 'card';
        // Determine if the preview is an image URL or an emoji.
        let previewHtml = '';
        if (item.preview && item.preview.startsWith('http') &&
           (item.preview.endsWith('.png') || item.preview.endsWith('.jpg') ||
            item.preview.endsWith('.jpeg') || item.preview.endsWith('.gif'))) {
          previewHtml = `<img src="${item.preview}" alt="preview" style="max-width:50px; max-height:50px; margin-right:10px;">`;
        } else if (item.preview) {
          previewHtml = `<span class="emoji">${item.preview}</span>`;
        }
        card.innerHTML = `<p>${previewHtml} <a href="${item.url}" target="_blank">${item.title}</a></p>`;
        container.appendChild(card);
      });
    }
    
    // Filter the data based on search input and state selection.
    function filterData() {
      const searchInputElem = document.getElementById('search-input');
      const searchInput = searchInputElem.value.trim().toLowerCase();
      const selectedState = document.getElementById('state-dropdown').value;
      const container = document.getElementById('data-container');

      // If both search input and state selection are empty, clear the results.
      if (searchInput === '' && selectedState === '') {
        container.innerHTML = '';
        return;
      }
      
      // Filter banned areas.
      let bannedResults = originalData.filter(item => {
        const matchState = (selectedState === '' || item.State === selectedState);
        const matchSearch = searchInput === '' || (
          (item.City && item.City.toLowerCase().includes(searchInput)) ||
          (item.County && item.County.toLowerCase().includes(searchInput)) ||
          (item.Zip && item.Zip.toLowerCase().includes(searchInput)) ||
          (item.State && item.State.toLowerCase().includes(searchInput))
        );
        return matchState && matchSearch;
      });
      
      // Filter supplemental data: if any tag matches the search input.
      let supplementalResults = supplementalData.filter(item => {
        if (!item.tags || !Array.isArray(item.tags)) return false;
        return item.tags.some(tag => tag.toLowerCase().includes(searchInput));
      });
      
      displayResults(bannedResults, supplementalResults);
    }
    
    // Set up event listeners.
    document.getElementById('search-input').addEventListener('input', filterData);
    document.getElementById('state-dropdown').addEventListener('change', filterData);
    
    // Initial data fetches.
    fetchData();
    fetchSupplemental();
  </script>
</body>
</html>
    "#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

/// Main entry point: start the Actix web server on port 7001.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Starting server at http://localhost:7001/");
    HttpServer::new(|| {
        App::new()
            .service(index)
            .service(data_handler)
            .service(supplemental_handler)
    })
    .bind(("127.0.0.1", 7001))?
    .run()
    .await
}
