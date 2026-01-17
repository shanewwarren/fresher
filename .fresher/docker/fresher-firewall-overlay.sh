#!/bin/bash
# .fresher/docker/fresher-firewall-overlay.sh
#
# Add custom domains to the devcontainer firewall whitelist.
# Run AFTER the standard init-firewall.sh has initialized the firewall.
#
# Usage:
#   1. Uncomment and modify the CUSTOM_DOMAINS array below
#   2. Add this script to postStartCommand in devcontainer.json:
#      "postStartCommand": "sudo /usr/local/bin/init-firewall.sh && ./.fresher/docker/fresher-firewall-overlay.sh"
#
# Common use cases:
#   - Private npm registry (e.g., npm.mycompany.com)
#   - Internal APIs (e.g., api.internal.mycompany.com)
#   - Private package registries (e.g., packages.mycompany.com)
#   - Internal git servers (e.g., git.mycompany.com)

# Uncomment and customize this array with your domains
# CUSTOM_DOMAINS=(
#   "npm.mycompany.com"
#   "api.internal-service.com"
#   "packages.mycompany.com"
# )

# Exit if CUSTOM_DOMAINS is not defined or empty
if [[ -z "${CUSTOM_DOMAINS[*]}" ]]; then
  echo "No custom domains configured. Edit this file to add domains."
  exit 0
fi

echo "Adding custom domains to firewall whitelist..."

for domain in "${CUSTOM_DOMAINS[@]}"; do
  # Resolve domain to IP addresses
  ips=$(dig +short A "$domain" 2>/dev/null)

  if [[ -z "$ips" ]]; then
    echo "Warning: Could not resolve $domain"
    continue
  fi

  for ip in $ips; do
    # Validate IP address format
    if [[ $ip =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
      sudo ipset add allowed-domains "$ip" 2>/dev/null || true
      echo "Added $domain ($ip) to whitelist"
    fi
  done
done

echo "Firewall overlay complete."
