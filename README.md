# GKCSearch

GKCSearch is a web dashboard for checking whether kratom is banned or regulated in a given area. It pulls ban data from a Google Sheets CSV (cached for 12 hours) and provides supplemental information (links and previews) based on area tags (zip codes, cities, or states). 

## Features

- **No Results by Default:**  
  The page initially shows no banned areas. If no matches are found, a “good news” message is displayed.

- **Search by City, County, or Zip:**  
  Use the search input (and optionally the state dropdown) to check if your area has any bans.

- **Match Summary Card:**  
  When a match is found, a card summarizes which field(s) (e.g., City, Zip) produced a match.

- **Supplemental Information:**  
  Supplemental links with previews (images or emojis) are shown when the search term matches tags from a supplemental configuration file (`supplemental.json`).

## Project Structure

```
GKCSearch/
├── Cargo.toml
├── src/
│   └── main.rs
├── supplemental.json
└── README.md
```

## How It Works

1. **Data Fetching:**  
   - The `/data` endpoint fetches and caches CSV data from a Google Sheets URL.
   - The `/supplemental` endpoint serves supplemental records from `supplemental.json`.

2. **Frontend:**  
   - The main page (`/`) loads a dark‑mode HTML page with a search input and state dropdown.
   - No results are shown until a search is performed.
   - When the user types in the search box (or selects a state), the client‑side script filters the data and displays:
     - A summary card if banned areas are found (including which fields matched).
     - Supplemental links (if any) based on matching tags.
   - If nothing is found, a “No kratom bans found in your area 🎉” message is shown.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs) (with Cargo)
- Internet connectivity (for fetching data from Google Sheets)
  
### Running Locally

1. Clone the repository (or create a new one as shown below).
2. Build and run the server:

   ```bash
   cargo run
   ```

3. Open your browser and navigate to [http://localhost:7001/](http://localhost:7001/).

## Creating and Pushing to GitHub from the CLI

Open your terminal in the project folder and run the following commands:

```bash
# Initialize a new git repository (if not already initialized)
git init

# Add all files to the repository
git add .

# Commit the files
git commit -m "Initial commit for GKCSearch MVP"

# Create a new public repository on GitHub using the GitHub CLI (make sure you have 'gh' installed)
gh repo create GKCSearch --public --source=. --remote=origin

# Push the commit to the master branch on GitHub
git push -u origin master
```

If you prefer not to use the GitHub CLI, you can manually create a repository named **GKCSearch** on GitHub and then run:

```bash
git remote add origin git@github.com:saintpetejackboy/GKCSearch.git
git push -u origin master
```

## License
This project is provided as-is for educational and prototyping purposes.
