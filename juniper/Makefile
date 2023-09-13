###############################
# Common defaults/definitions #
###############################

# Checks two given strings for equality.
eq = $(if $(or $(1),$(2)),$(and $(findstring $(1),$(2)),\
                                $(findstring $(2),$(1))),1)

# Multiplatform prefix of `sed -i` commands.
sed-i = sed -i$(if $(call eq,$(shell uname -s),Darwin), '',)




######################
# Project parameters #
######################

GRAPHIQL_VER ?= $(strip \
	$(shell grep -m1 '"graphiql": "' package.json | cut -d '"' -f4))




############
# Commands #
############

# Download and prepare actual version of GraphiQL static files, used for
# integrating it.
#
# Usage:
#	make graphiql

graphiql:
	curl -fL -o src/http/graphiql.html \
		https://raw.githubusercontent.com/graphql/graphiql/graphiql%40$(GRAPHIQL_VER)/examples/graphiql-cdn/index.html
	$(sed-i) 's|https://unpkg.com/graphiql/|https://unpkg.com/graphiql@$(GRAPHIQL_VER)/|g' \
		src/http/graphiql.html
	$(sed-i) "s|'https://swapi-graphql.netlify.app/.netlify/functions/index'|GRAPHQL_URL|g" \
		src/http/graphiql.html
	$(sed-i) "s|url: GRAPHQL_URL,|url: GRAPHQL_URL,\n        subscriptionUrl: normalizeSubscriptionEndpoint(GRAPHQL_URL, GRAPHQL_SUBSCRIPTIONS_URL)|" \
		src/http/graphiql.html
	$(sed-i) 's|<script>|<script>\n<!-- inject -->|' \
		src/http/graphiql.html
	$(sed-i) '/X-Example-Header/d' \
		src/http/graphiql.html




##################
# .PHONY section #
##################

.PHONY: graphiql
