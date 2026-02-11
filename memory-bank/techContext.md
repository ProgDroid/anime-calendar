# Tech Context: Anime Calendar

## Technologies Used
- **Backend**: Rust programming language with Actix Web framework
- **Frontend**: Vue.js 3 with TypeScript and Composition API
- **Styling**: Tailwind CSS for utility-first styling
- **Database**: SQLite with SQLx for database operations
- **API**: RESTful API with JWT authentication
- **External Services**: Anilist API for anime data
- **Build Tools**: Cargo for Rust, npm for frontend

## Development Setup
### Backend
- Rust toolchain (stable channel)
- Cargo package manager
- SQLite database
- Development server with hot-reload capabilities

### Frontend
- Node.js (LTS version)
- npm package manager
- Vue CLI or Vite for development server
- TypeScript compiler
- Tailwind CSS with PostCSS

## Technical Constraints
1. **Database**: Must use SQLite for simplicity and portability
2. **API**: Must follow REST conventions with proper HTTP status codes
3. **Authentication**: JWT-based authentication with secure token handling
4. **External API**: Rate limiting for Anilist API calls
5. **Responsive Design**: Must work on mobile, tablet, and desktop

## Dependencies
### Backend Dependencies
- actix-web: Web framework
- sqlx: Database toolkit
- serde: Serialization library
- dotenv: Environment variable management
-jsonwebtoken: JWT handling
- tokio: Async runtime

### Frontend Dependencies
- vue: Core framework
- vue-router: Routing
- axios: HTTP client
- tailwindcss: Styling
- typescript: Type safety

## Tool Usage Patterns
- **Rust Development**: Using Cargo for dependency management and building
- **Frontend Development**: Using npm scripts for building and serving
- **Database**: Using SQLx for type-safe database queries
- **Testing**: Unit tests for backend services, component tests for frontend
- **Deployment**: Container-based deployment with Docker

## Code Organization
### Backend Structure
- `src/` - Main source code
  - `controllers/` - HTTP request handlers
  - `services/` - Business logic
  - `entity/` - Domain models
  - `mappers/` - Data transformation
  - `database/` - Database operations
  - `middleware/` - Request processing

### Frontend Structure
- `src/` - Main source code
  - `components/` - Reusable UI components
  - `views/` - Page components
  - `services/` - API communication
  - `stores/` - State management
  - `router/` - Routing configuration
  - `assets/` - Static assets

## Configuration Management
- Environment variables for configuration
- Configuration files for different environments
- Type-safe configuration parsing

## Testing Strategy
- Unit testing for backend services
- Integration testing for API endpoints
- Component testing for frontend
- End-to-end testing for critical flows