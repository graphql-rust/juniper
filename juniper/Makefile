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
GRAPHQL_PLAYGROUND_VER ?= $(strip \
	$(shell grep -m1 '"graphql-playground-react": "' package.json | cut -d '"' -f4))




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
	$(sed-i) 's|unpkg.com/graphiql/|unpkg.com/graphiql@$(GRAPHIQL_VER)/|g' \
		src/http/graphiql.html
	$(sed-i) "s|'https://swapi-graphql.netlify.app/.netlify/functions/index'|JUNIPER_URL|g" \
		src/http/graphiql.html
	$(sed-i) "s|url: JUNIPER_URL,|url: JUNIPER_URL,\n        subscriptionUrl: normalizeSubscriptionEndpoint(JUNIPER_URL, JUNIPER_SUBSCRIPTIONS_URL)|" \
		src/http/graphiql.html
	$(sed-i) 's|<script>|<script>\n<!-- inject -->|' \
		src/http/graphiql.html
	$(sed-i) '/X-Example-Header/d' \
		src/http/graphiql.html


# Download and prepare actual version of GraphQL Playground static files, used
# for integrating it.
#
# Usage:
#	make graphql-playground

graphql-playground:
	curl -fL -o src/http/playground.html \
		https://raw.githubusercontent.com/graphql/graphql-playground/graphql-playground-react%40$(GRAPHQL_PLAYGROUND_VER)/packages/graphql-playground-html/withAnimation.html
	$(sed-i) 's|cdn.jsdelivr.net/npm/graphql-playground-react/|cdn.jsdelivr.net/npm/graphql-playground-react@$(GRAPHQL_PLAYGROUND_VER)/|g' \
		src/http/playground.html
	$(sed-i) "s|// you can add more options here|endpoint: 'JUNIPER_URL', subscriptionEndpoint: 'JUNIPER_SUBSCRIPTIONS_URL'|" \
		src/http/playground.html




##################
# .PHONY section #
##################

.PHONY: graphiql graphql-playground
