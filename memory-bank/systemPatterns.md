# System Patterns: Anime Calendar

## System Architecture
The Anime Calendar follows a standard three-tier architecture:
- **Presentation Layer**: Vue.js frontend with TypeScript and Tailwind CSS
- **Application Layer**: Rust backend with Actix Web framework
- **Data Layer**: SQLite database with ORM patterns

## Key Technical Decisions
1. **Backend Framework**: Rust with Actix Web for performance and memory safety
2. **Frontend Framework**: Vue.js with TypeScript for reactive UI components
3. **Database**: SQLite for simplicity and ease of deployment
4. **API Design**: RESTful API with JWT authentication
5. **External Integration**: Anilist API for anime data retrieval

## Design Patterns in Use
1. **MVC Pattern**: Backend follows Model-View-Controller structure
2. **Repository Pattern**: Database operations abstracted through repository interfaces
3. **Service Layer**: Business logic separated from controllers
4. **Mapper Pattern**: Data transformation between different layers (API, database, domain)
5. **Middleware Pattern**: Authentication and error handling middleware

## Component Relationships
- **Frontend Components**: Communicate with backend API through HTTP requests
- **Backend Controllers**: Handle HTTP requests and delegate to services
- **Services**: Contain business logic and coordinate between components
- **Mappers**: Transform data between different formats (database, API, domain objects)
- **Database**: Stores user data, calendar data, and anime items

## Critical Implementation Paths
1. **User Authentication Flow**: Registration → Login → JWT Token Management
2. **Calendar Management**: Create → View → Update → Delete calendars
3. **Anime Integration**: Search → Retrieve → Add to Calendar
4. **Data Synchronization**: Real-time updates and offline capabilities

## Data Flow Patterns
1. **Request Flow**: Frontend → API → Services → Database
2. **Response Flow**: Database → Services → API → Frontend
3. **Error Handling**: Centralized error handling with proper HTTP status codes
4. **Authentication Flow**: JWT token validation in middleware

## Performance Considerations
- Caching strategies for frequently accessed anime data
- Efficient database queries for calendar views
- Lazy loading for large calendar datasets
- API rate limiting for Anilist integration
