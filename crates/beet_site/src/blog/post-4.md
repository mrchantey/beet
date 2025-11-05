+++
title="The Full Moon Harvest #4"
created="2025-10-07"
+++

# The Full Moon Harvest #4

Action time!

<iframe width="941" height="538" src="https://www.youtube.com/embed/yI9tuBsrW1M" title="Full Moon Harvest #4 | Action Time" frameborder="0" allow="accelerometer; autoplay; clipboard-write; encrypted-media; gyroscope; picture-in-picture; web-share" referrerpolicy="strict-origin-when-cross-origin" allowfullscreen></iframe>



<br/>
<br/>

This one's a bit of a groundwork harvest, I have some exciting ideas for where to take beet next but to do it properly we need a firm base to launch from.

## Action Time

The Bevy `0.17` release introduces event triggers, which I used as an opportunity to dramatically clean up the `beet_flow` integration, leaning into a simple request-response format. `beet_flow` will play a critical role in the next pass at `beet_router`, which is gonna be a behavior tree router! cant wait.


## Webdriver BiDi

`beet_net` now has full cross-platform support for websockets, and we have an initial integration of Webdriver BiDi, the new standard set to replace Chrome DevTools Protocol. This will allow testing in an actual browser instead of just the html output (which in fairness has served pretty well so far!).

## Analytics

If a user experiences an error in a forest and no developer was around to hear, did it happen? yes, yes it did.
Analytics get a bad rap for creepy misuse but are an essential tool for any production application, even documentation sites can gain valuable insights from knowing how far users get into a walkthrough before closing the page.
This release has a first pass at sending analytics, click some counters in the home page and open the network tab to see your PII getting sold on the dark web BWAHAHA lol jks GDPR compliant, no user tracking, no annoying cookie banners.

Also partial initial support for DynamoDb which will likely be the place where this data is going to be stored. For now it lives locally in `target/analytics`.

## PDF Exports

Preparing for a conference i didnt want to copy all my beetmash stuff into a word doc and do formatting all over again so instead worked on print-to-pdf utilities like page breaks, hidden-unless-printing etc.


## Preparing for `bsn!`

Its been super handy getting to see carts progress with bsn, this harvest I've made some changes to alighn the templating system with the direction of bsn and the native bevy templating system. Its too early to say how much ill be able to deprecate (100% would be a dream), but the closer I can get the model the more we'll be able to do.

Thats about it for this harvest, all of these features put us in a steady position for having a lot of fun in the coming weeks.
