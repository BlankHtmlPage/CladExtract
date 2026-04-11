# Language info
language-name = English (United States)

# Tabs
music = Music
sounds = Sounds
images = Images
rbxm-files = RBXM Files
ktx-files = KTX Files
settings = Settings
about = About
logs = Logs

# Buttons
button-extract-type = Extract All of This Type <F3>
button-refresh = Refresh <F5>
button-clear-cache = Clear Cache <Del>
button-extract-all = Extract All <F3>
button-change-cache-dir = Change Cache Directory
button-reset-cache-dir = Reset Cache Directory
button-change-sql-db = Change SQL Database
button-reset-sql-db = Reset SQL Database
button-change-rbx-storage-dir = Change rbx-storage Directory
button-reset-rbx-storage-dir = Reset rbx-storage Directory
button-finish = Finish
button-yes = Yes
button-no = No
button-rename = Rename <F2>
button-search = Search <Ctrl+F>
button-swap = Swap Assets <F4>
button-copy-logs = Copy Log to Clipboard
button-export-logs = Export Log to File
button-copy = Copy <Ctrl+C>
button-open = Open <Return>
button-extract-file = Extract <Ctrl+E>
button-display-image-preview = Show Image Previews
button-disable-display-image-preview = Hide Image Previews
input-preview-size = Preview Size

# Confirmations
confirmation-filter-confirmation-title = Filtering in Progress
confirmation-filter-confirmation-description = Are you sure you want to extract files while filtering is still running? This may result in an incomplete extraction.
confirmation-clear-cache-title = Clear Cache
confirmation-clear-cache-description = Are you sure you want to clear the cache? Cached files will be regenerated the next time the client loads.
confirmation-custom-directory-title = Select Custom Directory
confirmation-custom-directory-description = Select a different directory to use as the cache?
confirmation-custom-sql-title = Select SQL Database
confirmation-custom-sql-description = Select a different SQLite database file?
confirmation-ban-warning-title = Ban Warning
confirmation-ban-warning-description = Modifying game assets can alter client behavior and may result in account or game bans. Use at your own risk. Do you wish to proceed?

# Errors
no-files = No files found.
error-directory-detection-title = Directory Detection Failed
error-directory-detection-description = Failed to detect the target directory. Ensure the client is installed and has been run at least once.
error-sql-detection-title = Database Detection Failed
error-sql-detection-description = Failed to detect the SQLite database. Ensure the client is installed and has been run at least once.
error-invalid-directory-title = Invalid Directory
error-invalid-directory-description = The provided path is not a valid directory. Please verify the path and try again.
error-invalid-database-title = Invalid Database
error-invalid-database-description = The provided path is not a valid SQLite database file. Please verify the file and try again.
generic-error-critical = Critical Error

# Headings
actions = Actions
updates = Updates
language-settings = Language Settings
new-updates = Updates Available
contributors = Contributors
dependencies = Dependencies
behavior = Behavior

# Checkboxes
check-for-updates = Check for Updates
automatically-install-updates = Automatically Install Updates
use-alias = Export Custom Filenames
use-topbar-buttons = Show Toolbar
refresh-before-extract = Refresh File List Before Extracting
download-development-build = Use Development Builds (Early access to features; may be unstable)
checkbox-hide-user-logs = Hide Username in Logs

# Descriptions
clear-cache-description = If file listing or extraction is running slowly, clear the cache using the button below. This removes cached data, which the client will automatically regenerate when needed.
extract-all-description = Extracts all assets and organizes them into categorized folders (e.g., /sounds, /images). You can select the destination folder when prompted.
custom-cache-dir-description = Change the cache directory below to use a custom location. You can revert to the default location using the reset button. Note: This is separate from the client's installation folder.
custom-sql-db-description = Select a custom SQLite database file below. You can revert to the default using the reset button. Note: This is separate from the client's installation folder.
custom-rbx-storage-dir-description = Specify a custom rbx-storage directory below. Use the reset button to restore the default path.
use-alias-description = When enabled, exports will use your custom renamed filenames instead of the original asset names. Rename files within the application to use this feature.
swap-choose-file = Double-click a file to swap.
swap-with = Double-click a file to swap with "{ $asset }"
logs-description = Application logs and error details are displayed here.
copy-choose-file = Double-click a file to copy.
overwrite-with = Double-click a file to overwrite "{ $asset }"

# Statuses
idling = Idling
deleting-files = Deleting files ({ $item }/{ $total })
extracting-files = Extracting files ({ $item }/{ $total })
filtering-files = Filtering files ({ $item }/{ $total })
all-extracted = Extraction Complete
swapped = Swapped "{ $item_a }" with "{ $item_b }"
copied = Overwrote "{ $item_b }" with "{ $item_a }"

# Error Statuses
failed-deleting-file = Failed to delete file ({ $item }/{ $total })
failed-opening-file = Failed to open file
error-check-logs = Check logs for additional details.

# Misc
version = Version: v{ $version } (compiled at { $date })
cache-directory = Cache Directory: { $directory }
sql-database = SQL Database: { $path }
rbx-storage-directory = rbx-storage Directory: { $directory }
no-directory = Not Found
welcome = Welcome
download-update-question = Would you like to download the update?
update-changelog = View the update changelog below:
setting-below-restart-required = Note: Changes to this setting require a restart to take effect.