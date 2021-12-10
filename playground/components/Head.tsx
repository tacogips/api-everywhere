import Head from "next/head";
import * as React from "react";
import { NextSeo } from "next-seo";

const HtmlHead: React.FC<{}> = ({}) => {
  return (
    <>
      <NextSeo
        title={"API Everywhere"}
        description={"Turn google spread sheet to json api"}
        openGraph={{
          title: "API Everywhere",
          description: "Turn google spread sheet to json api",
          site_name: "API Everywhere",
        }}
        twitter={{
          cardType: "summary",
        }}
      />
    </>
  );
};

export default HtmlHead;
