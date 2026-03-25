# Uncle - AI Playground

Goals:
- [ ] Generate or edit images using AI models.
- [ ] Chat-like interface like ChatGPT for conversational interactions with the AI.

## Tech Stack

- Axum and Askama for the web server and templating.
- Yaas API for single sign-on and user management.
- OpenAI API for AI interactions.
- AWS S3 for image input/output storage.
- Bulma CSS for multi-page application styling of the website.
- React and Tailwind CSS for the AI Playground interface.

## Models

image_prompts:
- id
- user_id
- prompt
- short_title
- model
- background
- moderation
- qty
- output_compression
- output_format
- quality
- status (pending|completed|failed)
- created_at
- updated_at

images:
- id
- user_id
- prompt_id
- category (input|output)
- filename
- file_type
- file_size
- file_path
- dimensions
- created_at

jobs:
- id
- job_type
- prompt_id
- status
- created_at
- updated_at

## Image Workflow

- User submits a prompt through the web interface along with attached images if there are any.
- Images are uploaded to AWS S3 using a presigned URL.
- Prompt is received in the server, stored in the database with pending status and returns.
- User periodically checks the status of the prompt until it is completed.
- Once completed, user fetches the generated images and displays them in the interface.
- If failed, user is notified and can try again or edit the prompt.

### Job Queue

- On submit of a prompt, a job is created.
- A worker process picks up pending jobs and processes then sequentially.
- Once job is completed, prompt status is updated.

## DB Migrations

```
tursodb db/uncle.db < migrations/migration-file.sql
```

## API

API endpoints:

- [ ] GET `/api/image-prompts` - List all image prompts for the current user.
- [ ] POST `/api/image-prompts` - Create a new image prompt.
- [ ] GET `/api/image-prompts/{id}` - Get details of a specific image prompt.
- [ ] GET `/api/image-prompts/{id}/status` - Get the status of a specific image prompt.
- [ ] POST `/api/images/upload-urls` - Get presigned URLs for uploading images to S3.
- [ ] POST `/api/image-prompts/{id}/images` - Add a new image related to the image prompt.
- [ ] GET `/api/image-prompts/{id}/images` - Get all images associated with a specific image prompt.

Response envelope:

- Success: `{ status_code: u16, message: String }`
- Error: `{ status_code: u16, message: String }`
