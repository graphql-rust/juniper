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
	$(eval graphiql-plugin-explorer-ver := $(strip \
		$(shell curl -fsL https://raw.githubusercontent.com/graphql/graphiql/refs/tags/graphiql%40$(GRAPHIQL_VER)/packages/graphiql-plugin-explorer/package.json \
		        | grep -m1 '"version": "' | cut -d '"' -f4)))
	$(eval graphiql-react-ver := $(strip \
		$(shell curl -fsL https://raw.githubusercontent.com/graphql/graphiql/refs/tags/graphiql%40$(GRAPHIQL_VER)/packages/graphiql/package.json \
		        | grep -m1 '"@graphiql/react": "' | cut -d '"' -f4 | cut -d '^' -f2)))
	curl -fsL -o src/http/graphiql.html \
		https://raw.githubusercontent.com/graphql/graphiql/graphiql%40$(GRAPHIQL_VER)/examples/graphiql-cdn/index.html
	$(sed-i) 's|esm.sh/graphiql@5.0.0-rc.1|esm.sh/graphiql@$(GRAPHIQL_VER)|g' \
		src/http/graphiql.html
	$(sed-i) 's|esm.sh/graphiql@6.0.0-canary-b26dcda1.0|esm.sh/graphiql@$(GRAPHIQL_VER)|g' \
		src/http/graphiql.html
	$(sed-i) 's|esm.sh/@graphiql/plugin-explorer@5.0.0-rc.1|esm.sh/@graphiql/plugin-explorer@$(graphiql-plugin-explorer-ver)|g' \
		src/http/graphiql.html
	$(sed-i) 's|esm.sh/@graphiql/react@0.36.0-canary-b26dcda1.0|esm.sh/@graphiql/react@$(graphiql-react-ver)|g' \
		src/http/graphiql.html
	$(sed-i) "s|'https://countries.trevorblades.com'|JUNIPER_URL|g" \
		src/http/graphiql.html
	$(sed-i) 's|url: JUNIPER_URL,|url: JUNIPER_URL,\n        subscriptionUrl: normalizeSubscriptionEndpoint(JUNIPER_URL, JUNIPER_SUBSCRIPTIONS_URL)|' \
		src/http/graphiql.html
	$(sed-i) 's|<script type="module">|<script type="module">\n<!-- inject -->|' \
		src/http/graphiql.html
	$(sed-i) 's|GraphiQL 5 with React 19 and GraphiQL Explorer|GraphiQL|' \
		src/http/graphiql.html


# Download and prepare actual version of GraphQL Playground static files, used
# for integrating it.
#
# Usage:
#	make graphql-playground

graphql-playground:
	curl -fsL -o src/http/playground.html \
		https://raw.githubusercontent.com/graphql/graphql-playground/graphql-playground-react%40$(GRAPHQL_PLAYGROUND_VER)/packages/graphql-playground-html/withAnimation.html
	$(sed-i) 's|cdn.jsdelivr.net/npm/graphql-playground-react/|cdn.jsdelivr.net/npm/graphql-playground-react@$(GRAPHQL_PLAYGROUND_VER)/|g' \
		src/http/playground.html
	$(sed-i) "s|// you can add more options here|endpoint: 'JUNIPER_URL', subscriptionEndpoint: 'JUNIPER_SUBSCRIPTIONS_URL'|" \
		src/http/playground.html




##################
# .PHONY section #
##################

.PHONY: graphiql graphql-playground
