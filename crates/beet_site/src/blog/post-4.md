+++
title="The Full Moon Harvest #4"
draft=true
created="2025-10-07"
+++

# The Full Moon Harvest #4

Action time!

<br/>
<br/>

This one's a bit of a groundwork harvest, I have some exciting ideas for where to take beet next but to do it properly we need a firm base to launch from.

## Webdriver BiDi

`beet_net` now has full cross-platform support for websockets, and we have an initial integration of Webdriver BiDi, the new standard set to replace Chrome DevTools Protocol. This will allow testing in an actual browser instead of just the html output (which has served pretty well so far!).

## Analytics

If a user experiences an error in a forest and no developer was around to hear, did it happen? yes, yes it did.
Analytics get a bad rap for creepy misuse but are an essential tool for any production application, even documentation sites can gain valuable insights from knowing how far users get into a walkthrough before closing the page.
This release has a first pass at sending analytics, check it out in the network tab!