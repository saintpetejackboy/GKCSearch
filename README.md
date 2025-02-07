# GKC Kratom Bans

[GKC Kratom Bans](https://github.com/saintpetejackboy/GKC-Kratom-Bans) is a Rust-based web service that retrieves, processes, and caches data on kratom bans. The application fetches CSV data from a publicly accessible Google Sheet, converts it into JSON, and serves it via a dynamic web interface built using Actix Web. Developed by [saintpetejackboy](https://github.com/saintpetejackboy), this project offers interactive search, drill-down views, and supplemental information to help users explore banned areas.

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Usage](#usage)
- [API Endpoints](#api-endpoints)
- [Project Structure](#project-structure)
- [Contributing](#contributing)
- [Disclaimer](#disclaimer)
- [License](#license)
- [Contact](#contact)

## Features

- **CSV Data Fetching & Processing:**  
  Retrieves data from a public Google Sheet (via CSV export) and converts it to JSON.
  
- **Caching Mechanism:**  
  Implements a caching system that stores processed JSON data for 12 hours to reduce unnecessary network requests.

- **Dynamic API Endpoints:**  
  - **`/data`**: Serves the processed JSON data of banned areas.
  - **`/supplemental`**: Serves supplemental JSON information (links, previews, tags) from a local file.
  - **`/`**: Serves the main interactive HTML/JS/CSS page.

- **Interactive Frontend:**  
  A modern, responsive web interface with:
  - A search panel that auto-updates the state selection based on user input.
  - Drill-down functionality to view banned areas by state, city, and zip code.
  - Animated visual elements and smooth transitions.

## Installation

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (Edition 2018 or later)
- Cargo (comes with Rust)
- A stable internet connection (to fetch CSV data from Google Sheets)

### Clone the Repository

Clone the project repository to your local machine:

```bash
git clone https://github.com/saintpetejackboy/GKC-Kratom-Bans.git
cd GKC-Kratom-Bans
```

### Build the Project

Build the project in release mode for optimized performance:

```bash
cargo build --release
```

## Usage

### Running the Server

Start the server using Cargo:

```bash
cargo run --release
```

By default, the server will start at [http://127.0.0.1:7001/](http://127.0.0.1:7001/). Open this URL in your web browser to access the application.

### How It Works

1. **Data Fetching & Caching:**  
   The backend fetches CSV data from a public Google Sheet, auto-detects the CSV delimiter, converts it to JSON, and caches it locally in `data_cache.json` for 12 hours.

2. **Supplemental Data:**  
   Additional info (e.g., links, previews, tags) is loaded from a `supplemental.json` file and served through the `/supplemental` endpoint.

3. **Interactive User Interface:**  
   The main page (`/`) presents a search panel and dynamic results area where users can:
   - Search by state, city, or zip code.
   - Auto-update the state dropdown based on the search input.
   - Drill down from state to city to view banned zip codes.
   - View supplemental information with clickable links and previews.

## API Endpoints

- **GET `/`**  
  Returns the main HTML page that includes the complete interactive UI.

- **GET `/data`**  
  Returns processed JSON data representing banned areas. This data is fetched from the Google Sheet, processed, and cached.

- **GET `/supplemental`**  
  Returns supplemental JSON data from the local `supplemental.json` file.

## Project Structure

```
.
├── Cargo.toml             # Project manifest with dependencies
├── src
│   └── main.rs            # Main Rust source file containing all server logic and endpoints
├── supplemental.json      # Supplemental information used by the `/supplemental` endpoint
└── data_cache.json        # Cached JSON data (generated automatically on first fetch)
```

## Contributing

Contributions are welcome! If you'd like to contribute to **GKC Kratom Bans**, please follow these steps:

1. **Fork the repository.**
2. **Create a new branch:**  
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Commit your changes:**  
   ```bash
   git commit -am "Add some feature"
   ```
4. **Push to your branch:**  
   ```bash
   git push origin feature/your-feature-name
   ```
5. **Create a Pull Request:**  
   Open a pull request detailing your changes.

Please ensure your code adheres to the project's style guidelines and includes relevant tests.

## Disclaimer

This service is provided for entertainment purposes only and is **not** a substitute for legal advice. For the most up-to-date legal information, please consult a qualified lawyer.

## License

*This project is provided for educational and informational purposes only. Please refer to the LICENSE file for details on licensing (if available), or contact the project owner for more information.*

## Contact

Developed by [saintpetejackboy](https://github.com/saintpetejackboy).  
For any questions, issues, or suggestions, please open an issue in the repository or contact me directly via GitHub.
