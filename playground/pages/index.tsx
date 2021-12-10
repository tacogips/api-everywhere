import * as React from "react";
import axios from "axios";
import { AxiosRequestConfig } from "axios";
import ParameterArea from "~/components/ParameterArea";
import Head from "~/components/Head";
import Header from "~/components/Header";
import Footer from "~/components/Footer";
import ResponseArea from "~/components/ResponseArea";
import { useState } from "react";

type Props = {};

const serverConfig = {
  url: process.env.NEXT_PUBLIC_SERVER_URL || "",
};

interface SheetMeta {
  data: {
    sheet_id_or_name: {
      tab_sheet_id: null | number;
      tab_sheet_name: null | string;
    };
    spread_sheet_id: string;
  };
}

const Home: React.FC<Props> = () => {
  let [responseVal, setResponseVal] = useState({
    apiURL: null,
    errorMessage: null,
    responseCode: null,
    responseBody: null,
  });

  let [searching, setSearching] = useState(false);

  const clearResult = () => {
    setResponseVal({
      apiURL: null,
      errorMessage: null,
      responseCode: null,
      responseBody: null,
    });
  };

  const setErrorMessage = (error: any) => {
    setResponseVal({
      errorMessage: error.toString(),
      apiURL: null,
      responseCode: null,
      responseBody: null,
    });
  };

  const getSheetMetaFromUrl = async (sheetUrl: string) => {
    if (!sheetUrl || sheetUrl.trim() === "") {
      return null;
    }
    clearResult();
    const encodedSheetUrl = encodeURIComponent(sheetUrl);

    const params = {
      sheet_url: encodedSheetUrl,
    };

    const config = {
      url: `${serverConfig.url}/sheet_meta`,
      method: "get",
      params,
    } as AxiosRequestConfig;

    const result = await axios.request(config).catch(error => {
      if (error.response) {
        if (error.response.status == 404) {
          setErrorMessage("api server not found ");
        } else if (error.response.status != 200) {
          setErrorMessage("sheet url is invalid");
        }
      }
      return null;
    });

    if (result) {
      return result.data;
    } else {
      return null;
    }
  };

  const isNonNegativeInt = (s: string): boolean => {
    const n = parseInt(s);
    if (n != NaN && n >= 0) {
      return true;
    }
    return false;
  };

  const getSheetDataFromUrl = async ({ data }, option) => {
    let params = {};
    if (data.sheet_id_or_name.tab_sheet_name != null) {
      params["sheet_name"] = data.sheet_id_or_name.tab_sheet_name;
    }
    if (data.sheet_id_or_name.tab_sheet_id != null) {
      params["sheet_id"] = data.sheet_id_or_name.tab_sheet_id;
    }

    if (isNonNegativeInt(option.offset)) {
      params["offset"] = parseInt(option.offset);
    }

    if (isNonNegativeInt(option.limit)) {
      params["limit"] = parseInt(option.limit);
    }

    if (isNonNegativeInt(option.row)) {
      params["row"] = parseInt(option.row);
    }

    const config = {
      url: `${serverConfig.url}/sheet/${data.spread_sheet_id}`,
      method: "get",
      params,
    } as AxiosRequestConfig;

    const apiUrlParams = new URLSearchParams(params);
    let apiUrl = `${serverConfig.url}/sheet/${data.spread_sheet_id}?${apiUrlParams}`;
    if (!serverConfig.url.startsWith("http")) {
      if (window) {
        const locationHref = [location.protocol, "//", location.host].join("");

        apiUrl = `${locationHref}/sheet/${data.spread_sheet_id}?${apiUrlParams}`;
      }
    }

    const result = await axios.request(config).catch(error => {
      if (error.response) {
        if (error.response.status == 404) {
          setResponseVal({
            errorMessage: null,
            apiURL: apiUrl,
            responseCode: error.response.status,
            responseBody: error.response.data,
          });
        } else if (error.response.status != 200) {
          setResponseVal({
            errorMessage: null,
            apiURL: apiUrl,
            responseCode: error.response.status,
            responseBody: error.response.data,
          });
        }
      }
      return null;
    });

    if (result) {
      setResponseVal({
        errorMessage: null,
        apiURL: apiUrl,
        responseCode: result.status,
        responseBody: result.data,
      });
    }

    return result;
  };

  const fetchData = async (
    sheetUrl: string,
    option: { offset: string; limit: string; row: string },
  ) => {
    setSearching(true);
    let metaResult: SheetMeta | null = await getSheetMetaFromUrl(sheetUrl);
    if (!metaResult) {
      setSearching(false);
      return;
    }

    await getSheetDataFromUrl(metaResult, option);

    setSearching(false);
  };

  return (
    <>
      <Head />
      <Header />
      <ParameterArea fetchData={fetchData} searching={searching} />
      <ResponseArea {...responseVal} />
      <Footer />
    </>
  );
};
export default Home;
