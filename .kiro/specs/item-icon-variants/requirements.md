# Requirements Document

## Introduction

This feature enables users to manage multiple icon variants for individual items within a program. Items can have multiple visual representations (e.g., "Health Potion|Original", "Health Potion|Ice", "Health Potion|Bone") while maintaining a single logical item entry in the program data. Icon variants are stored as separate PNG files using the ProgramDelimiter naming convention and managed through the editor UI.

## Glossary

- **Item**: A logical entity within a program that represents a game object with properties like name, grid size, tags, stack max, and merchant
- **Icon Variant**: A visual representation of an item, stored as a PNG file with a variant suffix (e.g., "Health Potion|Ice.png")
- **ProgramDelimiter**: The pipe character ("|") used to separate the base item name from its variant identifier
- **Item Grid**: The UI accordion display showing all unique items for a program, excluding variant duplicates
- **Editor Screen**: The UI interface where users manage program data including items, points, and search areas
- **Item Repository**: The data access layer managing CRUD operations for items within a program
- **Base Item Name**: The portion of an item name before the ProgramDelimiter (e.g., "Health Potion" from "Health Potion|Ice")

## Requirements

### Requirement 1

**User Story:** As a user, I want to add multiple icon variants for a single item, so that I can recognize different visual representations of the same item during automation.

#### Acceptance Criteria

1. WHEN the user creates an icon file with the format "{ItemName}|{VariantName}.png", THE Item Repository SHALL store the icon in the "internal/assets/images/icons/{ProgramName}/" directory
2. WHEN the user views the item editor, THE Editor Screen SHALL display a section for managing icon variants for the selected item
3. WHEN the user adds a new icon variant, THE Item Repository SHALL validate that the file exists in the correct directory with the correct naming format
4. THE Item Repository SHALL support a minimum of one icon variant and a maximum of 100 icon variants per item
5. WHEN the user removes an icon variant, THE Item Repository SHALL delete the corresponding PNG file from the filesystem

### Requirement 2

**User Story:** As a user, I want the item grid to show only one instance of each unique item regardless of how many icon variants exist, so that the UI remains clean and organized.

#### Acceptance Criteria

1. WHEN the Editor Screen displays the item grid, THE Item Grid SHALL group all icon variants by their base item name
2. WHEN multiple icon variants exist for an item (e.g., "Health Potion|Original", "Health Potion|Ice"), THE Item Grid SHALL display only one entry labeled with the base item name
3. THE Item Repository SHALL extract the base item name by parsing text before the ProgramDelimiter character
4. WHEN the user selects an item in the grid, THE Editor Screen SHALL load all associated icon variants in the editor panel
5. THE Item Grid SHALL maintain alphabetical sorting of items by their base item name

### Requirement 3

**User Story:** As a user, I want icon variants to be independent of the item data structure, so that adding or removing variants does not affect item properties or counts.

#### Acceptance Criteria

1. THE Item Repository SHALL store item properties (name, grid size, tags, stack max, merchant) independently from icon variant files
2. WHEN the user adds or removes an icon variant, THE Item Repository SHALL NOT modify the item count
3. WHEN the user queries item data, THE Item Repository SHALL return item properties without including icon variant information
4. THE Item Repository SHALL maintain backward compatibility with existing items that have no icon variants
5. WHEN an item has zero icon variants, THE Item Grid SHALL display the item with a placeholder icon

### Requirement 4

**User Story:** As a user, I want to add icon variants through the editor UI, so that I can manage all item-related data in one place.

#### Acceptance Criteria

1. WHEN the user selects an item in the editor, THE Editor Screen SHALL display a list of existing icon variants for that item
2. THE Editor Screen SHALL provide an "Add Icon Variant" button in the item editor panel
3. WHEN the user clicks "Add Icon Variant", THE Editor Screen SHALL open a file picker dialog filtered to PNG files
4. WHEN the user selects a PNG file, THE Item Repository SHALL copy the file to "internal/assets/images/icons/{ProgramName}/" with the naming format "{ItemName}|{VariantName}.png"
5. THE Editor Screen SHALL prompt the user to enter a variant name when adding a new icon

### Requirement 5

**User Story:** As a user, I want to remove icon variants through the editor UI, so that I can clean up unused or incorrect variants.

#### Acceptance Criteria

1. WHEN the user views an item's icon variants, THE Editor Screen SHALL display a delete button next to each variant
2. WHEN the user clicks the delete button, THE Editor Screen SHALL prompt for confirmation before deletion
3. WHEN the user confirms deletion, THE Item Repository SHALL remove the PNG file from the filesystem
4. WHEN the user deletes an icon variant, THE Editor Screen SHALL refresh the variant list in the editor
5. THE Editor Screen SHALL prevent deletion when only one icon variant remains for an item

### Requirement 6

**User Story:** As a user, I want to preview icon variants in the editor, so that I can verify I'm managing the correct visual assets.

#### Acceptance Criteria

1. WHEN the user views an item's icon variants, THE Editor Screen SHALL display a thumbnail preview of each icon with dimensions of 64x64 pixels
2. THE Editor Screen SHALL load icon images from "internal/assets/images/icons/{ProgramName}/{ItemName}|{VariantName}.png"
3. WHEN an icon file is missing or corrupted, THE Editor Screen SHALL display a placeholder image with an error indicator
4. THE Editor Screen SHALL scale icon thumbnails to 64x64 pixels while maintaining aspect ratio
5. THE Editor Screen SHALL display the variant name as a tooltip for each icon thumbnail
