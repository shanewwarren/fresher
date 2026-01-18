# Fresher Docker Image
# Based on Node.js with Claude Code CLI installed

FROM node:20-bookworm

# Install system dependencies
RUN apt-get update && apt-get install -y \
    git \
    jq \
    curl \
    bash \
    && rm -rf /var/lib/apt/lists/*

# Install Claude Code CLI globally
RUN npm install -g @anthropic-ai/claude-code

# Create non-root user if not using node user
RUN mkdir -p /home/node/.claude && chown -R node:node /home/node

# Switch to non-root user
USER node

# Set working directory
WORKDIR /workspace

# Default command
CMD ["/workspace/.fresher/run.sh"]
