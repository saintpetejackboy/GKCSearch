use actix_web::{get, App, HttpResponse, HttpServer, Responder};
use serde_json::{json, Value};
use csv::{ReaderBuilder, StringRecord};
use std::cmp::min;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use std::error::Error;

// ---------------------------------------------------------------------------
// Backend: CSV fetching, processing, and caching
// ---------------------------------------------------------------------------

/// Fetch the CSV data from Google Sheets and convert it to JSON.
async fn fetch_sheet_data_from_google() -> Result<Value, Box<dyn Error>> {
    // Google Sheet CSV export URL – ensure your sheet is publicly accessible.
    let sheet_url = "https://docs.google.com/spreadsheets/d/18kCz2igidQVgqwLdpsDA15kYXLxqX99r/export?format=csv&gid=1370952005";
    let response = reqwest::get(sheet_url).await?.text().await?;
    
    println!(
        "Raw CSV response (first 500 chars): {}",
        &response[..min(response.len(), 500)]
    );
    
    // Remove any potential BOM.
    let response = response.trim_start_matches('\u{feff}');
    
    // Auto-detect delimiter by comparing commas and semicolons in the first line.
    let first_line = response.lines().next().unwrap_or("");
    let comma_count = first_line.matches(',').count();
    let semicolon_count = first_line.matches(';').count();
    let delimiter = if semicolon_count > comma_count { b';' } else { b',' };
    println!("Detected delimiter: '{}'", delimiter as char);
    
    // Build CSV reader without headers.
    let mut rdr = ReaderBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .flexible(true)
        .from_reader(response.as_bytes());
    
    let mut header_record: Option<StringRecord> = None;
    let mut records = Vec::new();
    
    for result in rdr.records() {
        let record = result?;
        // Skip empty rows.
        if record.iter().all(|f| f.trim().is_empty()) {
            continue;
        }
        // Look for the header row (the proper header appears when the second field is "Zip").
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
    
    // Remove unwanted keys.
    for rec in records.iter_mut() {
        if let Value::Object(map) = rec {
            map.remove("Country");
            map.remove("column_0");
        }
    }
    
    Ok(json!(records))
}

/// Cache file path and duration (12 hours).
const CACHE_FILE: &str = "data_cache.json";
const CACHE_DURATION: Duration = Duration::from_secs(12 * 60 * 60);

/// Fetch the sheet data with caching.
async fn fetch_sheet_data() -> Result<Value, Box<dyn Error>> {
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
    
    println!("Fetching fresh data from Google Sheets...");
    let json_data = fetch_sheet_data_from_google().await?;
    
    // Save fresh data to cache.
    let json_string = serde_json::to_string_pretty(&json_data)?;
    let mut file = fs::File::create(CACHE_FILE).await?;
    file.write_all(json_string.as_bytes()).await?;
    println!("Saved new data to cache.");
    
    Ok(json_data)
}

// ---------------------------------------------------------------------------
// API endpoints
// ---------------------------------------------------------------------------

/// Endpoint to return banned area data as JSON.
#[get("/data")]
async fn data_handler() -> impl Responder {
    match fetch_sheet_data().await {
        Ok(json_data) => HttpResponse::Ok().json(json_data),
        Err(e) => HttpResponse::InternalServerError().body(format!("Error: {}", e)),
    }
}

/// Endpoint to return supplemental info (links, previews, tags) from JSON.
#[get("/supplemental")]
async fn supplemental_handler() -> impl Responder {
    match fs::read_to_string("supplemental.json").await {
        Ok(data) => match serde_json::from_str::<Value>(&data) {
            Ok(json_data) => HttpResponse::Ok().json(json_data),
            Err(e) => HttpResponse::InternalServerError()
                .body(format!("Error parsing supplemental JSON: {}", e)),
        },
        Err(e) => HttpResponse::InternalServerError()
            .body(format!("Error reading supplemental JSON file: {}", e)),
    }
}

/// The root endpoint (/) serves the complete HTML/JS/CSS page.
#[get("/")]
async fn index() -> impl Responder {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <title>GKC Kratom Bans 🌌</title>
  <style>
    /* Global reset and smooth transitions */
    * { box-sizing: border-box; margin: 0; padding: 0; }
    
    /* Subtle animated background */
    @keyframes backgroundAnimation {
      0% { background-position: 0% 50%; }
      50% { background-position: 100% 50%; }
      100% { background-position: 0% 50%; }
    }
    body {
      min-height: 100vh;
      font-family: 'Roboto', sans-serif;
      color: #e0e0e0;
      background: linear-gradient(135deg, #1e1e2f, #2e2e48);
      background-size: 200% 200%;
      animation: backgroundAnimation 20s ease infinite;
      transition: background 0.5s ease;
      display: flex;
      flex-direction: column;
    }
    
    /* Global link styles: new default and visited colors remain the same */
    a {
      color: #66ccff;
    }
    a:visited {
      color: #66ccff;
    }
    
    header {
      background: linear-gradient(135deg, #27293d, #1e1e2f);
      padding: 20px;
      text-align: center;
      font-size: 1.8em;
      font-weight: bold;
      box-shadow: 0 2px 4px rgba(0,0,0,0.3);
      transition: opacity 0.5s ease;
      /* Subtle pulsing animation */
      animation: pulse 3s ease-in-out infinite;
    }
    @keyframes pulse {
      0% { transform: scale(1); }
      50% { transform: scale(1.02); }
      100% { transform: scale(1); }
    }
    
    main {
      flex: 1;
      padding: 20px;
      max-width: 1200px;
      width: 100%;
      margin: 0 auto;
    }
    
    /* Search panel styling */
    .search-panel {
      display: flex;
      flex-wrap: wrap;
      gap: 10px;
      justify-content: center;
      margin-bottom: 10px;
      transition: opacity 0.5s ease;
    }
    .search-panel input[type="text"],
    .search-panel select {
      padding: 10px;
      border-radius: 5px;
      border: 1px solid #444;
      background: rgba(44, 47, 58, 0.9);
      color: #e0e0e0;
      font-size: 1em;
      min-width: 200px;
      transition: box-shadow 0.2s ease, opacity 0.5s ease;
    }
    .search-panel input[type="text"]:focus,
    .search-panel select:focus {
      box-shadow: 0 0 8px rgba(0,170,255,0.7);
      outline: none;
    }
    .search-panel button {
      background: linear-gradient(135deg, #00aaff, #005fbb);
      border: none;
      border-radius: 5px;
      padding: 10px 15px;
      color: #fff;
      font-size: 1em;
      cursor: pointer;
      box-shadow: 0 4px 6px rgba(0,0,0,0.2);
      transition: transform 0.2s, box-shadow 0.2s, opacity 0.5s ease;
    }
    .search-panel button:hover {
      transform: translateY(-2px);
      box-shadow: 0 6px 8px rgba(0,0,0,0.3);
    }
    
    /* Disclaimer styling */
    .disclaimer {
      font-size: 0.75em;
      margin: 10px 0;
      color: #ccc;
      padding: 10px;
      border: 1px solid #555;
      border-radius: 5px;
      background: rgba(0,0,0,0.5);
      opacity: 1;
      transition: opacity 1s ease;
    }
    
    /* Containers for results */
    .results, .drilldown-container, .supplemental-container {
      margin-top: 20px;
      padding: 10px;
      border-radius: 8px;
      background: rgba(44, 47, 58, 0.95);
      box-shadow: 0 2px 6px rgba(0,0,0,0.3);
      max-height: 300px;
      overflow-y: auto;
      transition: opacity 0.5s ease;
    }
    
    .card {
      background: rgba(44, 47, 58, 0.95);
      border: 1px solid #444;
      border-radius: 8px;
      padding: 15px;
      margin-bottom: 10px;
      transition: background 0.2s, transform 0.2s, opacity 0.5s ease;
    }
    .card:hover {
      background: rgba(58, 61, 75, 0.95);
      transform: translateY(-2px);
    }
    
    .drilldown-list {
      list-style: none;
      padding: 0;
      margin: 0;
    }
    .drilldown-list li {
      padding: 8px 10px;
      border-bottom: 1px solid #444;
      cursor: pointer;
      transition: background 0.2s, opacity 0.5s ease;
    }
    .drilldown-list li:hover {
      background: rgba(58, 61, 75, 0.9);
    }
    
    /* Flashing red cross for banned zip codes */
    .flashing {
      color: red;
      font-weight: bold;
      animation: flash 1s infinite;
    }
    @keyframes flash {
      0%, 50%, 100% { opacity: 1; }
      25%, 75% { opacity: 0; }
    }
    
    /* Success message styling */
    .success {
      background: rgba(20, 100, 20, 0.8);
      color: #d0ffd0;
      border: 1px solid #0f7a0f;
    }
    
    /* Supplemental info styling */
    .supplemental-card {
      background: rgba(44, 47, 58, 0.95);
      border: 1px solid #444;
      border-radius: 8px;
      padding: 10px;
      margin-bottom: 10px;
      display: flex;
      align-items: center;
      cursor: pointer;
      transition: transform 0.2s, opacity 0.5s ease;
    }
    .supplemental-card:hover {
      transform: translateY(-2px);
      opacity: 0.8;
    }
    
    footer {
      background: #27293d;
      text-align: center;
      padding: 10px;
      font-size: 0.8em;
      transition: opacity 0.5s ease;
    }
    
    /* Scrollbar styling */
    ::-webkit-scrollbar {
      width: 10px;
    }
    ::-webkit-scrollbar-track {
      background: #2c2f3a;
    }
    ::-webkit-scrollbar-thumb {
      background: #555;
      border-radius: 5px;
    }
  </style>
</head>
<body>
  <header>GKC Kratom Bans 🌌</header>
  <main>
    <!-- Search panel -->
    <div class="search-panel">
      <select id="state-dropdown">
        <option value="">-- Select State --</option>
      </select>
      <input id="search-input" type="text" placeholder="Search by City, County, or Zip..." />
      <button id="reset-btn">Reset</button>
    </div>
    
    <!-- Disclaimer now appears right under the inputs -->
    <div id="disclaimer-text" class="disclaimer">
      <p>This service is provided for entertainment purposes only and is not a substitute for legal advice. Please consult a lawyer for the most up-to-date legal information.</p>
    </div>
    
    <!-- Banned results (drill-down & success messages) -->
    <div id="results-summary" class="results" style="display:none;"></div>
    <div id="drilldown-container" class="drilldown-container" style="display:none;"></div>
    
    <!-- Supplemental info container -->
    <div id="supplemental-container" class="supplemental-container" style="display:none;"></div>
  </main>
  <footer>
    &copy; 2025 Brinstar
  </footer>
  <script>
    let bannedData = [];
    let supplementalData = [];
    let currentDrillLevel = 'state'; // "state" => list cities; "city" => list zip codes
    let filteredData = [];
    let drillStack = [];

    // -------------------------------------------------------------------------
    // Data fetching
    // -------------------------------------------------------------------------
    async function fetchBannedData() {
      try {
        const response = await fetch('/data');
        bannedData = await response.json();
        populateStateDropdown();
      } catch (error) {
        console.error('Error fetching banned data:', error);
      }
    }

    async function fetchSupplementalData() {
      try {
        const response = await fetch('/supplemental');
        supplementalData = await response.json();
      } catch (error) {
        console.error('Error fetching supplemental data:', error);
      }
    }

    // Populate state dropdown with states present in bannedData.
    function populateStateDropdown() {
      const stateDropdown = document.getElementById('state-dropdown');
      const states = [...new Set(bannedData.map(item => item.State).filter(s => s))].sort();
      stateDropdown.innerHTML = '<option value="">-- Select State --</option>';
      states.forEach(state => {
        const option = document.createElement('option');
        option.value = state;
        option.textContent = state;
        stateDropdown.appendChild(option);
      });
    }

    // -------------------------------------------------------------------------
    // Auto-update state dropdown based on search input
    // -------------------------------------------------------------------------
    function checkAndAutoUpdateState() {
      const searchInputElem = document.getElementById('search-input');
      let query = searchInputElem.value.trim();
      if (!query) return;
      const upperQuery = query.toUpperCase();
      // If query is 2 characters and matches a state code from bannedData, auto-select it.
      const availableStates = [...new Set(bannedData.map(item => item.State).filter(s => s))];
      if(query.length === 2 && availableStates.includes(upperQuery)) {
        document.getElementById('state-dropdown').value = upperQuery;
        searchInputElem.value = "";
        updateResults();
        return;
      }
      // Check if query is a zip code (5 digits) that uniquely belongs to one state.
      if(query.length === 5 && /^\d{5}$/.test(query)) {
        const matches = bannedData.filter(item => item.Zip === query);
        const uniqueStates = [...new Set(matches.map(item => item.State))];
        if(uniqueStates.length === 1) {
          document.getElementById('state-dropdown').value = uniqueStates[0];
          searchInputElem.value = "";
          updateResults();
          return;
        }
      }
      // Check if query exactly matches a city name that belongs to one state.
      const cityMatches = bannedData.filter(item => item.City && item.City.toLowerCase() === query.toLowerCase());
      const uniqueCityStates = [...new Set(cityMatches.map(item => item.State))];
      if(cityMatches.length > 0 && uniqueCityStates.length === 1) {
        document.getElementById('state-dropdown').value = uniqueCityStates[0];
        searchInputElem.value = "";
        updateResults();
        return;
      }
    }

    // -------------------------------------------------------------------------
    // Drill-down handling and results update
    // -------------------------------------------------------------------------
    function resetDrillDown() {
      currentDrillLevel = 'state';
      drillStack = [];
      const drillDiv = document.getElementById('drilldown-container');
      drillDiv.innerHTML = '';
      drillDiv.style.display = 'none';
    }

    function updateResults() {
      const searchQuery = document.getElementById('search-input').value.trim().toLowerCase();
      const selectedState = document.getElementById('state-dropdown').value;
      
      // If user changes state, clear the search input.
      if (selectedState) {
        document.getElementById('search-input').value = "";
      } else {
        // Otherwise, check if we can auto-update the state dropdown.
        checkAndAutoUpdateState();
      }
      
      resetDrillDown();

      // Filter bannedData based on state and search.
      filteredData = bannedData.filter(item => {
        const matchesState = !selectedState || item.State === selectedState;
        const matchesSearch = !searchQuery || (
          (item.City && item.City.toLowerCase().includes(searchQuery)) ||
          (item.County && item.County.toLowerCase().includes(searchQuery)) ||
          (item.Zip && item.Zip.toLowerCase().includes(searchQuery)) ||
          (item.State && item.State.toLowerCase().includes(searchQuery))
        );
        return matchesState && matchesSearch;
      });

      // Compute supplemental matches:
      let supplementalMatches = [];
      if(selectedState) {
        supplementalMatches = supplementalData.filter(item => 
          item.State && item.State.toLowerCase() === selectedState.toLowerCase()
        );
      } else if(searchQuery.length >= 2) {
        supplementalMatches = supplementalData.filter(item => {
          return (item.tags && item.tags.some(tag => tag.toLowerCase().includes(searchQuery))) ||
                 (item.State && item.State.toLowerCase().includes(searchQuery)) ||
                 (item.City && item.City.toLowerCase().includes(searchQuery));
        });
      }

      const resultsSummary = document.getElementById('results-summary');
      
      if (filteredData.length > 0) {
        // Show banned area results.
        resultsSummary.innerHTML = `
          <div class="card">
            <p><strong>${filteredData.length}</strong> banned area${filteredData.length > 1 ? 's' : ''} found.</p>
          </div>`;
        resultsSummary.style.display = 'block';
        renderDrillDown();
      } else if (searchQuery.length >= 2) {
        // No banned areas found: show success message.
        resultsSummary.innerHTML = `
          <div class="card success">
            <p>✅ Congratulations! There do not appear to be bans near "<strong>${searchQuery}</strong>".</p>
            <p>Please note: This information is not legal advice. Consult a lawyer for the most up-to-date information.</p>
          </div>`;
        resultsSummary.style.display = 'block';
      } else {
        resultsSummary.style.display = 'none';
      }
      
      // Always display supplemental info if there are matches.
      const suppContainer = document.getElementById('supplemental-container');
      if (supplementalMatches.length > 0) {
        renderSupplemental(supplementalMatches);
        suppContainer.style.display = 'block';
      } else {
        suppContainer.style.display = 'none';
      }
    }

    function renderDrillDown() {
      const container = document.getElementById('drilldown-container');
      container.innerHTML = '';
      let grouping = {};
      if (currentDrillLevel === 'state') {
        // Group filteredData by City.
        filteredData.forEach(item => {
          if (item.City) {
            grouping[item.City] = grouping[item.City] || [];
            grouping[item.City].push(item);
          }
        });
      } else if (currentDrillLevel === 'city') {
        // Group by Zip within the selected city.
        const currentCity = drillStack[drillStack.length - 1];
        filteredData.filter(item => item.City === currentCity)
          .forEach(item => {
            if (item.Zip) {
              grouping[item.Zip] = grouping[item.Zip] || [];
              grouping[item.Zip].push(item);
            }
          });
      }

      const ul = document.createElement('ul');
      ul.className = 'drilldown-list';
      for (const key in grouping) {
        const li = document.createElement('li');
        if (currentDrillLevel === 'state') {
          li.textContent = key + ' (' + grouping[key].length + ' Banned Zip Code' + (grouping[key].length > 1 ? 's' : '') + ')';
        } else if (currentDrillLevel === 'city') {
          li.innerHTML = key + ' <span class="flashing">❌</span>';
        }
        li.onclick = () => {
          if (currentDrillLevel === 'state') {
            currentDrillLevel = 'city';
            drillStack.push(key);
            renderDrillDown();
          }
        };
        ul.appendChild(li);
      }

      // Add a back button if in the city drill level.
      if (currentDrillLevel === 'city') {
        const backBtn = document.createElement('button');
        backBtn.textContent = '← Back to Cities';
        backBtn.onclick = () => {
          currentDrillLevel = 'state';
          drillStack.pop();
          renderDrillDown();
        };
        container.appendChild(backBtn);
      }

      container.appendChild(ul);
      container.style.display = 'block';
    }

    // Render supplemental info using only the JSON data.
    function renderSupplemental(matches) {
      const container = document.getElementById('supplemental-container');
      container.innerHTML = '';
      matches.forEach(item => {
        // Create a supplemental card as an anchor so the whole card is clickable.
        const link = document.createElement('a');
        link.href = item.url;
        link.target = "_blank";
        link.style.textDecoration = 'none';
        link.style.display = 'block';
        
        const card = document.createElement('div');
        card.className = 'supplemental-card';
        
        let previewHtml = '';
        // If the preview value is a URL to an image:
        if (item.preview && item.preview.startsWith('http') &&
            (item.preview.endsWith('.png') || item.preview.endsWith('.jpg') ||
             item.preview.endsWith('.jpeg') || item.preview.endsWith('.gif'))) {
          previewHtml = `<img src="${item.preview}" alt="preview">`;
        } else if (item.preview) {
          previewHtml = `<span style="font-size:2em; margin-right:10px;">${item.preview}</span>`;
        }
        
        // Create a container for the text.
        const textDiv = document.createElement('div');
        textDiv.innerHTML = `<strong>${item.title || item.url}</strong>`;
        
        card.innerHTML = `<p>${previewHtml}</p>`;
        card.appendChild(textDiv);
        link.appendChild(card);
        container.appendChild(link);
      });
    }

    // -------------------------------------------------------------------------
    // Disclaimer behavior: Fade out once the user interacts.
    // -------------------------------------------------------------------------
    function hideDisclaimer() {
      const disclaimer = document.getElementById('disclaimer-text');
      if (disclaimer) {
        disclaimer.style.opacity = '0';
        setTimeout(() => {
          disclaimer.style.display = 'none';
        }, 1000);
      }
      document.removeEventListener('click', hideDisclaimer);
      document.removeEventListener('input', hideDisclaimer);
    }
    document.addEventListener('click', hideDisclaimer);
    document.addEventListener('input', hideDisclaimer);

    // -------------------------------------------------------------------------
    // Event listeners and initialization
    // -------------------------------------------------------------------------
    document.getElementById('search-input').addEventListener('input', () => {
      checkAndAutoUpdateState();
      updateResults();
    });
    document.getElementById('state-dropdown').addEventListener('change', () => {
      document.getElementById('search-input').value = "";
      updateResults();
    });
    document.getElementById('reset-btn').addEventListener('click', () => {
      document.getElementById('search-input').value = '';
      document.getElementById('state-dropdown').value = '';
      resetDrillDown();
      document.getElementById('results-summary').style.display = 'none';
      document.getElementById('supplemental-container').style.display = 'none';
      updateResults();
    });

    // Initial data fetches.
    fetchBannedData();
    fetchSupplementalData();
  </script>
</body>
</html>
"#;
    HttpResponse::Ok().content_type("text/html").body(html)
}

// ---------------------------------------------------------------------------
// Main: start the Actix Web server.
// ---------------------------------------------------------------------------
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
