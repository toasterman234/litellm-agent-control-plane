FROM python:3.12-slim

ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1

WORKDIR /app

COPY requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt

COPY src ./src
COPY entrypoint.sh ./
RUN chmod +x ./entrypoint.sh

RUN mkdir -p /data

ENV PORT=8080 \
    DB_PATH=/data/agents.db \
    DEFAULT_MODEL=anthropic:claude-sonnet-4-6 \
    PYDANTIC_DEEP_WORKDIR_ROOT=/data/workspaces

EXPOSE 8080

ENTRYPOINT ["/app/entrypoint.sh"]
